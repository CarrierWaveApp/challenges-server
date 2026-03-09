use sqlx::PgPool;

use crate::db::park_boundaries::{self, UnfetchedPark};
use crate::models::park_boundary::{ArcGisFeature, ArcGisResponse};

const PADUS_URL: &str = "https://services.arcgis.com/v01gqwM5QqNysAAi/arcgis/rest/services/Manager_Name_PADUS/FeatureServer/0";

/// Natural England ArcGIS FeatureServer for UK national park boundaries.
const NATURAL_ENGLAND_URL: &str = "https://services.arcgis.com/JJzESW51TqeY9uat/ArcGIS/rest/services/National_Parks_England/FeatureServer/0";

/// WDPA (World Database on Protected Areas) ArcGIS FeatureServer for international parks.
const WDPA_URL: &str = "https://services5.arcgis.com/Mj0hjvkNtV7NRhA7/arcgis/rest/services/WDPA_v0/FeatureServer/0";

/// Which data source to use for a given park.
enum DataSource {
    PadUs,
    NaturalEngland,
    Wdpa { iso3: &'static str },
}

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
                                    "Park boundaries: {} '{}' -> no match (loc={:?})",
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

/// Determine which data source to use based on POTA reference prefix.
fn data_source_for_park(reference: &str) -> DataSource {
    if reference.starts_with("G-")
        || reference.starts_with("GM-")
        || reference.starts_with("GW-")
        || reference.starts_with("GI-")
    {
        DataSource::NaturalEngland
    } else if reference.starts_with("I-") {
        DataSource::Wdpa { iso3: "ITA" }
    } else {
        DataSource::PadUs
    }
}

/// Fetch boundary for a single park, routing to the correct data source.
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
    match data_source_for_park(&park.reference) {
        DataSource::PadUs => fetch_boundary_padus(pool, client, park).await,
        DataSource::NaturalEngland => fetch_boundary_uk(pool, client, park).await,
        DataSource::Wdpa { iso3 } => fetch_boundary_wdpa(pool, client, park, iso3).await,
    }
}

// ─── US (PAD-US) ────────────────────────────────────────────────────────────

async fn fetch_boundary_padus(
    pool: &PgPool,
    client: &reqwest::Client,
    park: &UnfetchedPark,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    let state_abbrev = park.location_desc.as_deref().and_then(state_code_to_abbrev);

    // Strategy 1: Name + state matching
    let search_name = normalize_park_name(&park.name);
    if let Some(state) = &state_abbrev {
        match query_padus_by_name(client, &search_name, state).await {
            Ok(Some(feature)) => {
                save_feature(pool, park, &feature, "exact", "pad_us_4").await?;
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
        match query_padus_by_point(client, lon, lat).await {
            Ok(Some(feature)) => {
                save_feature(pool, park, &feature, "spatial", "pad_us_4").await?;
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
async fn query_padus_by_name(
    client: &reqwest::Client,
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
        PADUS_URL,
        urlencoded(&where_clause)
    );

    let features = fetch_arcgis_features(client, &url, "PAD-US name").await?;
    // Prefer Designation feature class over Fee
    Ok(pick_best_feature(features))
}

/// Query PAD-US by point intersection.
async fn query_padus_by_point(
    client: &reqwest::Client,
    lon: f64,
    lat: f64,
) -> Result<Option<ArcGisFeature>, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!(
        "{}/query?geometry={},{}&geometryType=esriGeometryPoint&spatialRel=esriSpatialRelIntersects&outFields=Loc_Nm,Unit_Nm,Mang_Name,Des_Tp,GIS_Acres,FeatClass&f=geojson&outSR=4326",
        PADUS_URL, lon, lat
    );

    let features = fetch_arcgis_features(client, &url, "PAD-US spatial").await?;
    Ok(pick_best_feature(features))
}

// ─── UK (Natural England) ───────────────────────────────────────────────────

async fn fetch_boundary_uk(
    pool: &PgPool,
    client: &reqwest::Client,
    park: &UnfetchedPark,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    let search_name = normalize_park_name(&park.name);

    // Strategy 1: Name matching against Natural England dataset
    match query_uk_by_name(client, &search_name).await {
        Ok(Some(feature)) => {
            save_feature(pool, park, &feature, "exact", "natural_england").await?;
            return Ok(Some("exact".to_string()));
        }
        Ok(None) => {}
        Err(e) => {
            tracing::warn!(
                "Park boundaries: {} UK name query failed: {}",
                park.reference,
                e
            );
        }
    }

    // Strategy 2: Spatial query (point-in-polygon)
    if let (Some(lat), Some(lon)) = (park.latitude, park.longitude) {
        match query_uk_by_point(client, lon, lat).await {
            Ok(Some(feature)) => {
                save_feature(pool, park, &feature, "spatial", "natural_england").await?;
                return Ok(Some("spatial".to_string()));
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(
                    "Park boundaries: {} UK spatial query failed: {}",
                    park.reference,
                    e
                );
            }
        }
    }

    Ok(None)
}

/// Query Natural England by park name.
async fn query_uk_by_name(
    client: &reqwest::Client,
    search_name: &str,
) -> Result<Option<ArcGisFeature>, Box<dyn std::error::Error + Send + Sync>> {
    let escaped_name = search_name.replace('\'', "''");
    let where_clause = format!("NAME LIKE '%{}%'", escaped_name);

    let url = format!(
        "{}/query?where={}&outFields=NAME,DESIG_DATE,AREA_HA&f=geojson&outSR=4326",
        NATURAL_ENGLAND_URL,
        urlencoded(&where_clause)
    );

    let features = fetch_arcgis_features(client, &url, "Natural England name").await?;
    Ok(features.into_iter().next())
}

/// Query Natural England by point intersection.
async fn query_uk_by_point(
    client: &reqwest::Client,
    lon: f64,
    lat: f64,
) -> Result<Option<ArcGisFeature>, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!(
        "{}/query?geometry={},{}&geometryType=esriGeometryPoint&spatialRel=esriSpatialRelIntersects&outFields=NAME,DESIG_DATE,AREA_HA&f=geojson&outSR=4326",
        NATURAL_ENGLAND_URL, lon, lat
    );

    let features = fetch_arcgis_features(client, &url, "Natural England spatial").await?;
    Ok(features.into_iter().next())
}

// ─── International (WDPA) ───────────────────────────────────────────────────

async fn fetch_boundary_wdpa(
    pool: &PgPool,
    client: &reqwest::Client,
    park: &UnfetchedPark,
    iso3: &str,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    let search_name = normalize_park_name(&park.name);
    let source = format!("wdpa_{}", iso3.to_lowercase());

    // Strategy 1: Name + country matching
    match query_wdpa_by_name(client, &search_name, iso3).await {
        Ok(Some(feature)) => {
            save_feature(pool, park, &feature, "exact", &source).await?;
            return Ok(Some("exact".to_string()));
        }
        Ok(None) => {}
        Err(e) => {
            tracing::warn!(
                "Park boundaries: {} WDPA name query failed: {}",
                park.reference,
                e
            );
        }
    }

    // Strategy 2: Spatial query (point-in-polygon) filtered by country
    if let (Some(lat), Some(lon)) = (park.latitude, park.longitude) {
        match query_wdpa_by_point(client, lon, lat, iso3).await {
            Ok(Some(feature)) => {
                save_feature(pool, park, &feature, "spatial", &source).await?;
                return Ok(Some("spatial".to_string()));
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(
                    "Park boundaries: {} WDPA spatial query failed: {}",
                    park.reference,
                    e
                );
            }
        }
    }

    Ok(None)
}

/// Query WDPA by name and ISO3 country code.
async fn query_wdpa_by_name(
    client: &reqwest::Client,
    search_name: &str,
    iso3: &str,
) -> Result<Option<ArcGisFeature>, Box<dyn std::error::Error + Send + Sync>> {
    let escaped_name = search_name.replace('\'', "''");
    let where_clause = format!(
        "NAME LIKE '%{}%' AND ISO3 = '{}'",
        escaped_name, iso3
    );

    let url = format!(
        "{}/query?where={}&outFields=NAME,DESIG_ENG,DESIG,IUCN_CAT,REP_AREA,ISO3&f=geojson&outSR=4326",
        WDPA_URL,
        urlencoded(&where_clause)
    );

    let features = fetch_arcgis_features(client, &url, "WDPA name").await?;
    Ok(features.into_iter().next())
}

/// Query WDPA by point intersection filtered by country.
async fn query_wdpa_by_point(
    client: &reqwest::Client,
    lon: f64,
    lat: f64,
    iso3: &str,
) -> Result<Option<ArcGisFeature>, Box<dyn std::error::Error + Send + Sync>> {
    let where_clause = format!("ISO3 = '{}'", iso3);

    let url = format!(
        "{}/query?geometry={},{}&geometryType=esriGeometryPoint&spatialRel=esriSpatialRelIntersects&where={}&outFields=NAME,DESIG_ENG,DESIG,IUCN_CAT,REP_AREA,ISO3&f=geojson&outSR=4326",
        WDPA_URL, lon, lat,
        urlencoded(&where_clause)
    );

    let features = fetch_arcgis_features(client, &url, "WDPA spatial").await?;
    Ok(features.into_iter().next())
}

// ─── Shared helpers ─────────────────────────────────────────────────────────

/// Fetch features from an ArcGIS REST endpoint, parsing the standard response format.
async fn fetch_arcgis_features(
    client: &reqwest::Client,
    url: &str,
    label: &str,
) -> Result<Vec<ArcGisFeature>, Box<dyn std::error::Error + Send + Sync>> {
    let resp_text = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let resp: ArcGisResponse = match serde_json::from_str(&resp_text) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(
                "Park boundaries: {} response parse error: {} (first 200 chars: {})",
                label,
                e,
                &resp_text[..resp_text.len().min(200)]
            );
            return Ok(vec![]);
        }
    };

    Ok(resp.features.unwrap_or_default())
}

/// Pick the best feature from a PAD-US result set (prefers Designation over Fee).
fn pick_best_feature(features: Vec<ArcGisFeature>) -> Option<ArcGisFeature> {
    features.into_iter().min_by_key(|f| {
        match f
            .properties
            .as_ref()
            .and_then(|a| a.feat_class.as_deref())
        {
            Some("Designation") => 0,
            Some("Fee") => 1,
            _ => 2,
        }
    })
}

/// Save an ArcGIS feature as a park boundary.
async fn save_feature(
    pool: &PgPool,
    park: &UnfetchedPark,
    feature: &ArcGisFeature,
    match_quality: &str,
    source: &str,
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
    let designation = attrs
        .and_then(|a| a.des_tp.as_deref())
        .or_else(|| attrs.and_then(|a| a.desig_eng.as_deref()))
        .or_else(|| attrs.and_then(|a| a.desig.as_deref()));
    let manager = attrs.and_then(|a| a.mang_name.as_deref());
    let acreage = attrs
        .and_then(|a| a.gis_acres)
        .or_else(|| attrs.and_then(|a| a.area_ha.map(|ha| ha * 2.47105)))
        .or_else(|| attrs.and_then(|a| a.rep_area.map(|km2| km2 * 247.105)));

    park_boundaries::upsert_boundary(
        pool,
        &park.reference,
        &park.name,
        designation,
        manager,
        acreage,
        match_quality,
        &geojson,
        source,
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

/// Normalize a POTA park name for search by stripping common suffixes.
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
        " Parco Nazionale",
        " Parco Regionale",
        " Riserva Naturale",
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
