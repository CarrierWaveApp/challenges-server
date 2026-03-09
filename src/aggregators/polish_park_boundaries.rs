use sqlx::PgPool;

use crate::db::park_boundaries::{self, UnfetchedPark};
use crate::models::park_boundary::{WfsFeature, WfsFeatureCollection};

/// GDOŚ WFS endpoint for Polish protected areas.
const GDOS_WFS_URL: &str = "https://sdi.gdos.gov.pl/wfs";

/// WFS layer names for different Polish protected area types.
const WFS_LAYERS: &[&str] = &[
    "ParkiNarodowe",
    "ParkiKrajobrazowe",
    "Rezerwaty",
    "ObszaryChronionegoKrajobrazu",
    "ObszarySpecjalnejOchrony",
    "SpecjalneObszaryOchrony",
];

/// Configuration for the Polish park boundaries aggregator.
pub struct PolishParkBoundariesConfig {
    pub batch_size: i64,
    pub cycle_hours: u64,
    pub stale_days: i64,
}

impl Default for PolishParkBoundariesConfig {
    fn default() -> Self {
        Self {
            batch_size: 20,
            cycle_hours: 24,
            stale_days: 90,
        }
    }
}

enum FetchResult {
    Cached(String),
    NoMatch,
    Error(String),
}

/// Main poll loop — fetches boundaries for unmatched Polish parks, then re-checks stale ones.
pub async fn poll_loop(pool: PgPool, client: reqwest::Client, config: PolishParkBoundariesConfig) {
    // Wait for POTA stats aggregator to populate park catalog
    tracing::info!("Polish park boundaries: waiting 180s for POTA stats to populate park catalog");
    tokio::time::sleep(std::time::Duration::from_secs(180)).await;

    loop {
        let total_cached = park_boundaries::count_boundaries(&pool).await.unwrap_or(0);

        // Phase 1: Fetch boundaries for Polish parks that don't have one yet
        match park_boundaries::get_unfetched_polish_parks(&pool, config.batch_size).await {
            Ok(parks) => {
                if parks.is_empty() {
                    tracing::info!(
                        "Polish park boundaries: all SP- parks fetched ({} total cached)",
                        total_cached
                    );
                } else {
                    tracing::info!(
                        "Polish park boundaries: fetching {} unfetched parks ({} total cached)",
                        parks.len(),
                        total_cached
                    );
                    let mut cached = 0u32;
                    let mut no_match = 0u32;
                    let mut errors = 0u32;

                    for park in &parks {
                        match fetch_boundary(&pool, &client, park).await {
                            FetchResult::Cached(quality) => {
                                tracing::info!(
                                    "Polish park boundaries: {} '{}' -> cached ({})",
                                    park.reference,
                                    park.name,
                                    quality
                                );
                                cached += 1;
                            }
                            FetchResult::NoMatch => {
                                tracing::info!(
                                    "Polish park boundaries: {} '{}' -> no match",
                                    park.reference,
                                    park.name,
                                );
                                no_match += 1;
                            }
                            FetchResult::Error(e) => {
                                tracing::warn!(
                                    "Polish park boundaries: {} '{}' -> error: {}",
                                    park.reference,
                                    park.name,
                                    e
                                );
                                errors += 1;
                            }
                        }
                        // Rate limit: 2 seconds between requests to be polite to GDOŚ
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    }

                    let new_total = park_boundaries::count_boundaries(&pool)
                        .await
                        .unwrap_or(0);
                    tracing::info!(
                        "Polish park boundaries: batch done — {} cached, {} no match, {} errors ({} total cached)",
                        cached,
                        no_match,
                        errors,
                        new_total
                    );
                }
            }
            Err(e) => {
                tracing::error!(
                    "Polish park boundaries: get_unfetched_polish_parks failed: {}",
                    e
                );
            }
        }

        // Phase 2: Re-fetch stale Polish boundaries
        match park_boundaries::get_stale_boundaries(&pool, config.stale_days, config.batch_size)
            .await
        {
            Ok(stale) => {
                // Only process SP- parks from the stale list
                let polish_stale: Vec<_> = stale
                    .into_iter()
                    .filter(|s| s.reference.starts_with("SP-"))
                    .collect();
                if !polish_stale.is_empty() {
                    tracing::info!(
                        "Polish park boundaries: refreshing {} stale boundaries",
                        polish_stale.len()
                    );
                    for park in &polish_stale {
                        let unfetched = UnfetchedPark {
                            reference: park.reference.clone(),
                            name: park.name.clone(),
                            location_desc: park.location_desc.clone(),
                            latitude: park.latitude,
                            longitude: park.longitude,
                        };
                        match fetch_boundary(&pool, &client, &unfetched).await {
                            FetchResult::Cached(quality) => {
                                tracing::info!(
                                    "Polish park boundaries: {} refreshed ({})",
                                    park.reference,
                                    quality
                                );
                            }
                            FetchResult::NoMatch => {
                                tracing::info!(
                                    "Polish park boundaries: {} refresh -> no match",
                                    park.reference
                                );
                            }
                            FetchResult::Error(e) => {
                                tracing::warn!(
                                    "Polish park boundaries: {} refresh failed: {}",
                                    park.reference,
                                    e
                                );
                            }
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    "Polish park boundaries: get_stale_boundaries failed: {}",
                    e
                );
            }
        }

        tracing::info!(
            "Polish park boundaries: sleeping {}h until next cycle",
            config.cycle_hours
        );
        tokio::time::sleep(std::time::Duration::from_secs(config.cycle_hours * 3600)).await;
    }
}

/// Fetch boundary for a single Polish park from GDOŚ WFS.
async fn fetch_boundary(
    pool: &PgPool,
    client: &reqwest::Client,
    park: &UnfetchedPark,
) -> FetchResult {
    match fetch_boundary_inner(pool, client, park).await {
        Ok(Some(quality)) => FetchResult::Cached(quality),
        Ok(None) => FetchResult::NoMatch,
        Err(e) => FetchResult::Error(e.to_string()),
    }
}

async fn fetch_boundary_inner(
    pool: &PgPool,
    client: &reqwest::Client,
    park: &UnfetchedPark,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    let search_name = normalize_polish_park_name(&park.name);

    // Strategy 1: Search by name across all WFS layers
    for layer in WFS_LAYERS {
        match query_wfs_by_name(client, layer, &search_name).await {
            Ok(Some(feature)) => {
                save_wfs_feature(pool, park, &feature, "exact", layer).await?;
                return Ok(Some("exact".to_string()));
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(
                    "Polish park boundaries: {} WFS name query on {} failed: {}",
                    park.reference,
                    layer,
                    e
                );
            }
        }
    }

    // Strategy 2: Spatial query (point-in-polygon) across all layers
    if let (Some(lat), Some(lon)) = (park.latitude, park.longitude) {
        for layer in WFS_LAYERS {
            match query_wfs_by_point(client, layer, lon, lat).await {
                Ok(Some(feature)) => {
                    save_wfs_feature(pool, park, &feature, "spatial", layer).await?;
                    return Ok(Some("spatial".to_string()));
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(
                        "Polish park boundaries: {} WFS spatial query on {} failed: {}",
                        park.reference,
                        layer,
                        e
                    );
                }
            }
        }
    } else {
        tracing::info!(
            "Polish park boundaries: {} has no lat/lon, skipping spatial query",
            park.reference
        );
    }

    Ok(None)
}

/// Query GDOŚ WFS by name using a CQL filter.
async fn query_wfs_by_name(
    client: &reqwest::Client,
    layer: &str,
    search_name: &str,
) -> Result<Option<WfsFeature>, Box<dyn std::error::Error + Send + Sync>> {
    let escaped_name = search_name.replace('\'', "''");
    let cql_filter = format!("nazwa LIKE '%{}%'", escaped_name);

    let url = format!(
        "{}?SERVICE=WFS&VERSION=2.0.0&REQUEST=GetFeature\
         &TYPENAMES=GDOS:{}\
         &CQL_FILTER={}\
         &outputFormat=application/json\
         &srsName=EPSG:4326\
         &COUNT=5",
        GDOS_WFS_URL,
        layer,
        urlencoded(&cql_filter)
    );

    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!(
            "WFS returned {} (first 200: {})",
            status,
            &body[..body.len().min(200)]
        )
        .into());
    }

    let resp_text = resp.text().await?;
    let collection: WfsFeatureCollection = match serde_json::from_str(&resp_text) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                "Polish park boundaries: WFS response parse error on {}: {} (first 200: {})",
                layer,
                e,
                &resp_text[..resp_text.len().min(200)]
            );
            return Ok(None);
        }
    };

    let features = collection.features.unwrap_or_default();
    // Return the first feature with geometry
    Ok(features.into_iter().find(|f| f.geometry.is_some()))
}

/// Query GDOŚ WFS by point intersection using a BBOX filter.
async fn query_wfs_by_point(
    client: &reqwest::Client,
    layer: &str,
    lon: f64,
    lat: f64,
) -> Result<Option<WfsFeature>, Box<dyn std::error::Error + Send + Sync>> {
    // Use a small bounding box around the point (~100m)
    let delta = 0.001;
    let bbox = format!(
        "{},{},{},{},EPSG:4326",
        lat - delta,
        lon - delta,
        lat + delta,
        lon + delta
    );

    let url = format!(
        "{}?SERVICE=WFS&VERSION=2.0.0&REQUEST=GetFeature\
         &TYPENAMES=GDOS:{}\
         &BBOX={}\
         &outputFormat=application/json\
         &srsName=EPSG:4326\
         &COUNT=5",
        GDOS_WFS_URL, layer, bbox
    );

    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!(
            "WFS returned {} (first 200: {})",
            status,
            &body[..body.len().min(200)]
        )
        .into());
    }

    let resp_text = resp.text().await?;
    let collection: WfsFeatureCollection = match serde_json::from_str(&resp_text) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                "Polish park boundaries: WFS spatial parse error on {}: {} (first 200: {})",
                layer,
                e,
                &resp_text[..resp_text.len().min(200)]
            );
            return Ok(None);
        }
    };

    let features = collection.features.unwrap_or_default();
    Ok(features.into_iter().find(|f| f.geometry.is_some()))
}

/// Save a WFS feature as a park boundary.
async fn save_wfs_feature(
    pool: &PgPool,
    park: &UnfetchedPark,
    feature: &WfsFeature,
    match_quality: &str,
    layer: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let geojson = match &feature.geometry {
        Some(g) => serde_json::to_string(g)?,
        None => {
            tracing::warn!(
                "Polish park boundaries: {} matched but feature has no geometry",
                park.reference
            );
            return Ok(());
        }
    };

    let props = feature.properties.as_ref();
    let area_ha = props.and_then(|p| p.area_ha);
    // Convert hectares to acres for consistency with US data (1 ha = 2.47105 acres)
    let acreage = area_ha.map(|ha| ha * 2.47105);

    let source = format!("gdos_wfs_{}", layer);

    park_boundaries::upsert_boundary(
        pool,
        &park.reference,
        &park.name,
        Some(layer), // Use the WFS layer name as designation
        Some("GDOŚ"),
        acreage,
        match_quality,
        &geojson,
        &source,
    )
    .await?;

    Ok(())
}

/// URL-encode a string for use in WFS query parameters.
fn urlencoded(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('\'', "%27")
        .replace('=', "%3D")
        .replace('&', "%26")
}

/// Normalize a Polish POTA park name for WFS search by stripping common suffixes.
fn normalize_polish_park_name(name: &str) -> String {
    let suffixes = [
        " - Park Narodowy",
        " Park Narodowy",
        " - Park Krajobrazowy",
        " Park Krajobrazowy",
        " - Rezerwat Przyrody",
        " Rezerwat Przyrody",
        " - Obszar Chronionego Krajobrazu",
        " Obszar Chronionego Krajobrazu",
        " - Zespół Przyrodniczo-Krajobrazowy",
        " Zespół Przyrodniczo-Krajobrazowy",
        " - Użytek Ekologiczny",
        " Użytek Ekologiczny",
        " - Pomnik Przyrody",
        " Pomnik Przyrody",
        " - Stanowisko Dokumentacyjne",
        " Stanowisko Dokumentacyjne",
        " National Park",
        " Landscape Park",
        " Nature Reserve",
    ];

    let mut result = name.to_string();
    for suffix in &suffixes {
        if let Some(stripped) = result.strip_suffix(suffix) {
            result = stripped.to_string();
            break;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_polish_park_name() {
        assert_eq!(
            normalize_polish_park_name("Białowieski Park Narodowy"),
            "Białowieski"
        );
        assert_eq!(
            normalize_polish_park_name("Dolina Baryczy - Park Krajobrazowy"),
            "Dolina Baryczy"
        );
        assert_eq!(
            normalize_polish_park_name("Some Reserve - Rezerwat Przyrody"),
            "Some Reserve"
        );
        // Name without a recognized suffix stays unchanged
        assert_eq!(
            normalize_polish_park_name("Kampinoski"),
            "Kampinoski"
        );
    }
}
