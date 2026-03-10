use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::Semaphore;

use crate::db::park_boundaries::{self, UnfetchedPark};
use crate::metrics as app_metrics;
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
    pub concurrency: usize,
}

impl Default for ParkBoundariesConfig {
    fn default() -> Self {
        Self {
            batch_size: 20,
            cycle_hours: 24,
            stale_days: 90,
            concurrency: 5,
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

    let semaphore = Arc::new(Semaphore::new(config.concurrency));

    loop {
        let batch_start = std::time::Instant::now();
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

                    let (cached, no_match, errors) =
                        fetch_batch(&pool, &client, &semaphore, parks).await;

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
                metrics::counter!(app_metrics::SYNC_ERRORS_TOTAL, "aggregator" => "park_boundaries")
                    .increment(1);
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
                    let unfetched: Vec<UnfetchedPark> = stale
                        .into_iter()
                        .map(|park| UnfetchedPark {
                            reference: park.reference,
                            name: park.name,
                            location_desc: park.location_desc,
                            latitude: park.latitude,
                            longitude: park.longitude,
                        })
                        .collect();

                    fetch_batch(&pool, &client, &semaphore, unfetched).await;
                }
            }
            Err(e) => {
                tracing::error!("Park boundaries: get_stale_boundaries failed: {}", e);
                metrics::counter!(app_metrics::SYNC_ERRORS_TOTAL, "aggregator" => "park_boundaries")
                    .increment(1);
            }
        }

        // Record batch metrics
        let new_total = park_boundaries::count_boundaries(&pool).await.unwrap_or(0);
        metrics::gauge!(app_metrics::GIS_BOUNDARIES_CACHED_TOTAL).set(new_total as f64);
        metrics::histogram!(app_metrics::GIS_BATCH_DURATION_SECONDS, "aggregator" => "park_boundaries")
            .record(batch_start.elapsed().as_secs_f64());
        metrics::gauge!(app_metrics::SYNC_LAST_COMPLETED_TIMESTAMP, "aggregator" => "park_boundaries")
            .set(chrono::Utc::now().timestamp() as f64);

        tracing::info!(
            "Park boundaries: sleeping {}h until next cycle",
            config.cycle_hours
        );
        tokio::time::sleep(std::time::Duration::from_secs(config.cycle_hours * 3600)).await;
    }
}

/// Fetch a batch of parks concurrently using the semaphore for rate limiting.
async fn fetch_batch(
    pool: &PgPool,
    client: &reqwest::Client,
    semaphore: &Arc<Semaphore>,
    parks: Vec<UnfetchedPark>,
) -> (u32, u32, u32) {
    let mut handles = Vec::with_capacity(parks.len());

    for park in parks {
        let pool = pool.clone();
        let client = client.clone();
        let semaphore = semaphore.clone();

        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            let start = std::time::Instant::now();
            let source_label = match data_source_for_park(&park.reference) {
                DataSource::PadUs => "pad_us",
                DataSource::NaturalEngland => "natural_england",
                DataSource::Wdpa { .. } => "wdpa",
            };
            let result = fetch_boundary(&pool, &client, &park).await;

            // Record no-match sentinel so the park doesn't block the queue
            if matches!(result, FetchResult::NoMatch) {
                if let Err(e) =
                    park_boundaries::upsert_no_match(&pool, &park.reference, &park.name, source_label)
                        .await
                {
                    tracing::error!(
                        "Park boundaries: {} failed to record no-match: {}",
                        park.reference,
                        e
                    );
                }
            }

            let result_label = match &result {
                FetchResult::Cached(_) => "cached",
                FetchResult::NoMatch => "no_match",
                FetchResult::Error(_) => "error",
            };
            metrics::counter!(app_metrics::GIS_FETCH_TOTAL, "source" => source_label.to_string(), "result" => result_label)
                .increment(1);
            metrics::histogram!(app_metrics::GIS_FETCH_DURATION_SECONDS, "source" => source_label.to_string())
                .record(start.elapsed().as_secs_f64());
            (park.reference, park.name, park.location_desc, result)
        });
        handles.push(handle);
    }

    let mut cached = 0u32;
    let mut no_match = 0u32;
    let mut errors = 0u32;

    for handle in handles {
        match handle.await {
            Ok((reference, name, location_desc, FetchResult::Cached(quality))) => {
                tracing::info!(
                    "Park boundaries: {} '{}' -> cached ({})",
                    reference,
                    name,
                    quality
                );
                cached += 1;
            }
            Ok((reference, name, location_desc, FetchResult::NoMatch)) => {
                tracing::info!(
                    "Park boundaries: {} '{}' -> no match (loc={:?})",
                    reference,
                    name,
                    location_desc
                );
                no_match += 1;
            }
            Ok((reference, name, _location_desc, FetchResult::Error(e))) => {
                tracing::warn!(
                    "Park boundaries: {} '{}' -> error: {}",
                    reference,
                    name,
                    e
                );
                errors += 1;
            }
            Err(e) => {
                tracing::error!("Park boundaries: task join error: {}", e);
                errors += 1;
            }
        }
    }

    (cached, no_match, errors)
}

/// Determine which data source to use based on POTA reference prefix.
fn data_source_for_park(reference: &str) -> DataSource {
    if reference.starts_with("GB-") {
        DataSource::NaturalEngland
    } else if reference.starts_with("IT-") {
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
    if let Some(state) = &state_abbrev {
        match query_padus_by_name(client, &park.name, state).await {
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
///
/// Tries the full park name first for a precise match. If nothing is returned,
/// falls back to the normalized (suffix-stripped) name with a designation-type
/// filter so that e.g. "Huron" doesn't pull in every golf course and metropark
/// in the state.
async fn query_padus_by_name(
    client: &reqwest::Client,
    full_name: &str,
    state: &str,
) -> Result<Option<ArcGisFeature>, Box<dyn std::error::Error + Send + Sync>> {
    let out_fields = "Loc_Nm,Unit_Nm,Mang_Name,Des_Tp,GIS_Acres,FeatClass";

    // Strategy A: exact match on full park name
    let escaped_full = full_name.replace('\'', "''");
    let where_full = format!(
        "Loc_Nm = '{}' AND State_Nm = '{}'",
        escaped_full, state
    );
    let url_full = format!(
        "{}/query?where={}&outFields={}&f=geojson&outSR=4326",
        PADUS_URL,
        urlencoded(&where_full),
        out_fields,
    );
    let features = fetch_arcgis_features(client, &url_full, "PAD-US exact name").await?;
    if !features.is_empty() {
        return Ok(merge_padus_features(features));
    }

    // Strategy B: normalized name + designation filter
    let normalized = normalize_park_name(full_name);
    if normalized == full_name {
        // No suffix was stripped — nothing more to try
        return Ok(None);
    }

    let escaped_norm = normalized.replace('\'', "''");
    let des_filter = designation_filter_for_name(full_name).unwrap_or_default();
    let where_norm = format!(
        "Loc_Nm LIKE '%{}%' AND State_Nm = '{}' {}",
        escaped_norm, state, des_filter,
    );
    let url_norm = format!(
        "{}/query?where={}&outFields={}&f=geojson&outSR=4326",
        PADUS_URL,
        urlencoded(&where_norm),
        out_fields,
    );
    let features = fetch_arcgis_features(client, &url_norm, "PAD-US normalized name").await?;
    // Merge all features into one — parks like Don Edwards SF Bay NWR (US-0189)
    // have many parcels that must be combined into a single geometry.
    Ok(merge_padus_features(features))
}

/// Return an additional WHERE clause fragment that constrains results by
/// PAD-US designation type or managing agency, based on the park name suffix.
fn designation_filter_for_name(name: &str) -> Option<&'static str> {
    let lower = name.to_lowercase();
    // Order matters — check longer suffixes first
    let mappings: &[(&str, &str)] = &[
        ("national wildlife refuge", "AND (Des_Tp IN ('NWR','MPA','WA') OR Mang_Name = 'FWS')"),
        ("national park and preserve", "AND (Des_Tp IN ('NP','NPRE','WA') OR Mang_Name = 'NPS')"),
        ("national park", "AND (Des_Tp IN ('NP','NPRE','WA') OR Mang_Name = 'NPS')"),
        ("national forest", "AND (Des_Tp = 'NF' OR Mang_Name = 'USFS')"),
        ("national recreation area", "AND (Des_Tp = 'NRA' OR Mang_Name IN ('NPS','USFS','BLM'))"),
        ("national monument", "AND (Des_Tp IN ('NM','NME') OR Mang_Name IN ('NPS','BLM'))"),
        ("national seashore", "AND (Des_Tp IN ('NS','NLS') OR Mang_Name = 'NPS')"),
        ("national lakeshore", "AND (Des_Tp IN ('NL','NLS') OR Mang_Name = 'NPS')"),
        ("state park", "AND Des_Tp = 'SP'"),
        ("state forest", "AND Des_Tp = 'SF'"),
        ("state recreation area", "AND Des_Tp IN ('SRMA','SCA')"),
        ("wilderness area", "AND Des_Tp = 'WA'"),
        ("wilderness", "AND Des_Tp IN ('WA','WSA')"),
        ("wildlife management area", "AND Des_Tp IN ('SCA','WA')"),
    ];
    for (suffix, filter) in mappings {
        if lower.ends_with(suffix) {
            return Some(filter);
        }
    }
    None
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
    // Merge spatial results too — a point can intersect multiple parcels of the same park
    Ok(merge_padus_features(features))
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

/// Query Natural England by park name — merges all returned features.
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
    Ok(merge_arcgis_features(features))
}

/// Query Natural England by point intersection — merges all returned features.
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
    Ok(merge_arcgis_features(features))
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

/// Query WDPA by name and ISO3 country code — merges all returned features.
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
    Ok(merge_arcgis_features(features))
}

/// Query WDPA by point intersection filtered by country — merges all returned features.
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
    Ok(merge_arcgis_features(features))
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

/// Merge all PAD-US features into a single feature with combined geometry.
///
/// Parks like Don Edwards San Francisco Bay NWR (US-0189) have many separate
/// parcels in PAD-US across multiple FeatClass categories (Designation, Fee,
/// Marine, etc.). We merge ALL geometries into a single GeometryCollection
/// so the full park boundary is preserved, and sum the total acreage.
fn merge_padus_features(features: Vec<ArcGisFeature>) -> Option<ArcGisFeature> {
    if features.is_empty() {
        return None;
    }
    if features.len() == 1 {
        return features.into_iter().next();
    }

    // Collect all geometries and sum acreage across all FeatClass categories
    let mut geometries = Vec::new();
    let mut total_acres: f64 = 0.0;
    for feature in &features {
        if let Some(geom) = &feature.geometry {
            geometries.push(geom.clone());
        }
        if let Some(acres) = feature.properties.as_ref().and_then(|a| a.gis_acres) {
            total_acres += acres;
        }
    }

    if geometries.is_empty() {
        return None;
    }

    let merged_geometry = merge_geojson_geometries(geometries);

    // Take attributes from the first feature, override acreage with total
    let mut result = features.into_iter().next().unwrap();
    result.geometry = Some(merged_geometry);
    if total_acres > 0.0 {
        if let Some(ref mut attrs) = result.properties {
            attrs.gis_acres = Some(total_acres);
        }
    }
    Some(result)
}

/// Merge all ArcGIS features (non-PAD-US) into a single feature with merged geometry.
/// Used for Natural England and WDPA queries where there's no FeatClass grouping.
fn merge_arcgis_features(features: Vec<ArcGisFeature>) -> Option<ArcGisFeature> {
    if features.is_empty() {
        return None;
    }
    if features.len() == 1 {
        return features.into_iter().next();
    }

    let geometries: Vec<serde_json::Value> = features
        .iter()
        .filter_map(|f| f.geometry.clone())
        .collect();

    if geometries.is_empty() {
        return None;
    }

    let merged_geometry = merge_geojson_geometries(geometries);
    let mut result = features.into_iter().next().unwrap();
    result.geometry = Some(merged_geometry);
    Some(result)
}

/// Merge multiple GeoJSON geometries into one.
/// Returns the single geometry if only one, or a GeometryCollection if multiple.
pub fn merge_geojson_geometries(geometries: Vec<serde_json::Value>) -> serde_json::Value {
    if geometries.len() == 1 {
        return geometries.into_iter().next().unwrap();
    }
    serde_json::json!({
        "type": "GeometryCollection",
        "geometries": geometries
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
/// The `%` must be encoded first (to `%25`) so that SQL LIKE wildcards
/// don't get misinterpreted as URL percent-encoding by the server.
fn urlencoded(s: &str) -> String {
    s.replace('%', "%25")
        .replace(' ', "%20")
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
