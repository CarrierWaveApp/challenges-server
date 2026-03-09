use sqlx::PgPool;

use crate::db::park_boundaries::{self, UnfetchedPark};
use crate::models::park_boundary::{ArcGisFeature, ArcGisResponse};

const PADUS_URL: &str = "https://services.arcgis.com/v01gqwM5QqNysAAi/arcgis/rest/services/Manager_Name_PADUS/FeatureServer/0";

/// Configuration for the park boundaries aggregator.
pub struct ParkBoundariesConfig {
    pub batch_size: i64,
    pub cycle_hours: u64,
    pub stale_days: i64,
}

impl Default for ParkBoundariesConfig {
    fn default() -> Self {
        Self {
            batch_size: 20,
            cycle_hours: 24,
            stale_days: 90,
        }
    }
}

/// Result of attempting to fetch a boundary for one park.
enum FetchResult {
    Cached(String), // match_quality
    NoMatch,
    Error(String),
}

/// Main poll loop — fetches boundaries for unmatched parks, then re-checks stale ones.
pub async fn poll_loop(pool: PgPool, client: reqwest::Client, config: ParkBoundariesConfig) {
    // Wait for POTA stats aggregator to populate pota_parks first
    tracing::info!("Park boundaries: waiting 120s for POTA stats to populate park catalog");
    tokio::time::sleep(std::time::Duration::from_secs(120)).await;

    loop {
        let total_cached = park_boundaries::count_boundaries(&pool).await.unwrap_or(0);

        // Phase 1: Fetch boundaries for parks that don't have one yet
        match park_boundaries::get_unfetched_parks(&pool, config.batch_size).await {
            Ok(parks) => {
                if parks.is_empty() {
                    tracing::info!(
                        "Park boundaries: all parks fetched ({} cached)",
                        total_cached
                    );
                } else {
                    tracing::info!(
                        "Park boundaries: fetching {} unfetched parks ({} already cached)",
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
                                    "Park boundaries: {} '{}' -> cached ({})",
                                    park.reference,
                                    park.name,
                                    quality
                                );
                                cached += 1;
                            }
                            FetchResult::NoMatch => {
                                tracing::info!(
                                    "Park boundaries: {} '{}' -> no match (state={:?})",
                                    park.reference,
                                    park.name,
                                    park.location_desc
                                );
                                no_match += 1;
                            }
                            FetchResult::Error(e) => {
                                tracing::warn!(
                                    "Park boundaries: {} '{}' -> error: {}",
                                    park.reference,
                                    park.name,
                                    e
                                );
                                errors += 1;
                            }
                        }
                        // Rate limit: 1 request per second to be polite to ArcGIS
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }

                    let new_total = park_boundaries::count_boundaries(&pool)
                        .await
                        .unwrap_or(0);
                    tracing::info!(
                        "Park boundaries: batch done — {} cached, {} no match, {} errors ({} total cached)",
                        cached,
                        no_match,
                        errors,
                        new_total
                    );
                }
            }
            Err(e) => {
                tracing::error!("Park boundaries: get_unfetched_parks failed: {}", e);
            }
        }

        // Phase 2: Re-fetch stale boundaries
        match park_boundaries::get_stale_boundaries(&pool, config.stale_days, config.batch_size)
            .await
        {
            Ok(stale) => {
                if !stale.is_empty() {
                    tracing::info!(
                        "Park boundaries: refreshing {} stale boundaries",
                        stale.len()
                    );
                    for park in &stale {
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
                                    "Park boundaries: {} refreshed ({})",
                                    park.reference,
                                    quality
                                );
                            }
                            FetchResult::NoMatch => {
                                tracing::info!(
                                    "Park boundaries: {} refresh -> no match",
                                    park.reference
                                );
                            }
                            FetchResult::Error(e) => {
                                tracing::warn!(
                                    "Park boundaries: {} refresh failed: {}",
                                    park.reference,
                                    e
                                );
                            }
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }
            Err(e) => {
                tracing::error!("Park boundaries: get_stale_boundaries failed: {}", e);
            }
        }

        tracing::info!(
            "Park boundaries: sleeping {}h until next cycle",
            config.cycle_hours
        );
        tokio::time::sleep(std::time::Duration::from_secs(config.cycle_hours * 3600)).await;
    }
}

/// Fetch boundary for a single park from PAD-US.
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
    let state_abbrev = park.location_desc.as_deref().and_then(state_code_to_abbrev);

    // Strategy 1: Name + state matching
    let search_name = normalize_park_name(&park.name);
    if let Some(state) = &state_abbrev {
        match query_by_name(client, PADUS_URL, &search_name, state).await {
            Ok(Some(feature)) => {
                save_feature(pool, park, &feature, "exact").await?;
                return Ok(Some("exact".to_string()));
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(
                    "Park boundaries: {} name query failed: {}",
                    park.reference,
                    e
                );
            }
        }
    } else {
        tracing::info!(
            "Park boundaries: {} has no state mapping for '{:?}', skipping name query",
            park.reference,
            park.location_desc
        );
    }

    // Strategy 2: Spatial query (point-in-polygon)
    if let (Some(lat), Some(lon)) = (park.latitude, park.longitude) {
        match query_by_point(client, PADUS_URL, lon, lat).await {
            Ok(Some(feature)) => {
                save_feature(pool, park, &feature, "spatial").await?;
                return Ok(Some("spatial".to_string()));
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(
                    "Park boundaries: {} spatial query failed: {}",
                    park.reference,
                    e
                );
            }
        }
    } else {
        tracing::info!(
            "Park boundaries: {} has no lat/lon, skipping spatial query",
            park.reference
        );
    }

    Ok(None)
}

/// Query PAD-US by name and state.
async fn query_by_name(
    client: &reqwest::Client,
    service_url: &str,
    search_name: &str,
    state: &str,
) -> Result<Option<ArcGisFeature>, Box<dyn std::error::Error + Send + Sync>> {
    let escaped_name = search_name.replace('\'', "''");
    let where_clause = format!(
        "Loc_Nm LIKE '%{}%' AND State_Nm = '{}'",
        escaped_name, state
    );

    let url = format!(
        "{}/query?where={}&outFields=Loc_Nm,Unit_Nm,Mang_Name,Des_Tp,GIS_Acres,FeatClass&f=geojson&outSR=4326",
        service_url,
        urlencoded(&where_clause)
    );

    let resp_text = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let resp: ArcGisResponse = match serde_json::from_str(&resp_text) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(
                "Park boundaries: ArcGIS response parse error: {} (first 200 chars: {})",
                e,
                &resp_text[..resp_text.len().min(200)]
            );
            return Ok(None);
        }
    };

    let features = resp.features.unwrap_or_default();
    // Prefer Designation feature class over Fee
    let best = features.into_iter().min_by_key(|f| {
        match f
            .properties
            .as_ref()
            .and_then(|a| a.feat_class.as_deref())
        {
            Some("Designation") => 0,
            Some("Fee") => 1,
            _ => 2,
        }
    });

    Ok(best)
}

/// Query PAD-US by point intersection.
async fn query_by_point(
    client: &reqwest::Client,
    service_url: &str,
    lon: f64,
    lat: f64,
) -> Result<Option<ArcGisFeature>, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!(
        "{}/query?geometry={},{}&geometryType=esriGeometryPoint&spatialRel=esriSpatialRelIntersects&outFields=Loc_Nm,Unit_Nm,Mang_Name,Des_Tp,GIS_Acres,FeatClass&f=geojson&outSR=4326",
        service_url, lon, lat
    );

    let resp_text = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let resp: ArcGisResponse = match serde_json::from_str(&resp_text) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(
                "Park boundaries: ArcGIS spatial response parse error: {} (first 200 chars: {})",
                e,
                &resp_text[..resp_text.len().min(200)]
            );
            return Ok(None);
        }
    };

    let features = resp.features.unwrap_or_default();
    let best = features.into_iter().min_by_key(|f| {
        match f
            .properties
            .as_ref()
            .and_then(|a| a.feat_class.as_deref())
        {
            Some("Designation") => 0,
            Some("Fee") => 1,
            _ => 2,
        }
    });

    Ok(best)
}

/// Save an ArcGIS feature as a park boundary.
async fn save_feature(
    pool: &PgPool,
    park: &UnfetchedPark,
    feature: &ArcGisFeature,
    match_quality: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let geojson = match &feature.geometry {
        Some(g) => serde_json::to_string(g)?,
        None => {
            tracing::warn!(
                "Park boundaries: {} matched but feature has no geometry",
                park.reference
            );
            return Ok(());
        }
    };

    let attrs = feature.properties.as_ref();
    let designation = attrs.and_then(|a| a.des_tp.as_deref());
    let manager = attrs.and_then(|a| a.mang_name.as_deref());
    let acreage = attrs.and_then(|a| a.gis_acres);

    park_boundaries::upsert_boundary(
        pool,
        &park.reference,
        &park.name,
        designation,
        manager,
        acreage,
        match_quality,
        &geojson,
        "pad_us_4",
    )
    .await?;

    Ok(())
}

/// URL-encode a string for use in ArcGIS REST API query parameters.
/// Note: `%` in SQL LIKE wildcards must NOT be encoded — ArcGIS expects
/// the `where` parameter to contain raw SQL with `%` wildcards.
fn urlencoded(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('\'', "%27")
        .replace('=', "%3D")
        .replace('&', "%26")
}

/// Normalize a POTA park name for PAD-US search by stripping common suffixes.
fn normalize_park_name(name: &str) -> String {
    let suffixes = [
        " National Park and Preserve",
        " National Park",
        " National Forest",
        " National Wildlife Refuge",
        " National Recreation Area",
        " National Monument",
        " National Seashore",
        " National Lakeshore",
        " State Park",
        " State Forest",
        " State Recreation Area",
        " Wilderness Area",
        " Wilderness",
        " Wildlife Management Area",
        " WMA",
        " State Natural Area",
        " State Historic Site",
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

/// Extract state abbreviation from POTA location code.
/// Handles single-state codes like "US-ME" -> "ME".
/// Returns None for multi-state codes like "US-DC,US-MD,US-WV".
fn state_code_to_abbrev(code: &str) -> Option<&str> {
    // Skip multi-state codes (contain commas)
    if code.contains(',') {
        return None;
    }
    code.strip_prefix("US-")
}
