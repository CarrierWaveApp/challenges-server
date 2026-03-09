use sqlx::PgPool;

use crate::db::park_boundaries::{self, UnfetchedPark};
use crate::models::park_boundary::{ArcGisFeature, ArcGisResponse};

const PADUS_FEDERAL_URL: &str = "https://services.arcgis.com/P3ePLMYs2RVChkJx/arcgis/rest/services/Protected_Areas_by_Manager_Federal/FeatureServer/0";
const PADUS_STATE_URL: &str = "https://services.arcgis.com/P3ePLMYs2RVChkJx/arcgis/rest/services/Protected_Areas_by_Manager_State/FeatureServer/0";

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

/// Main poll loop — fetches boundaries for unmatched parks, then re-checks stale ones.
pub async fn poll_loop(pool: PgPool, client: reqwest::Client, config: ParkBoundariesConfig) {
    // Wait for POTA stats aggregator to populate pota_parks first
    tokio::time::sleep(std::time::Duration::from_secs(120)).await;

    loop {
        // Phase 1: Fetch boundaries for parks that don't have one yet
        match park_boundaries::get_unfetched_parks(&pool, config.batch_size).await {
            Ok(parks) => {
                if parks.is_empty() {
                    tracing::debug!("Park boundaries: no unfetched parks");
                } else {
                    tracing::info!(
                        "Park boundaries: fetching {} unfetched parks",
                        parks.len()
                    );
                    for park in &parks {
                        if let Err(e) = fetch_boundary(&pool, &client, park).await {
                            tracing::warn!(
                                "Park boundaries: {} fetch failed: {}",
                                park.reference,
                                e
                            );
                        }
                        // Rate limit: 1 request per second to be polite to ArcGIS
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
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
                    tracing::info!("Park boundaries: refreshing {} stale boundaries", stale.len());
                    for park in &stale {
                        let unfetched = UnfetchedPark {
                            reference: park.reference.clone(),
                            name: park.name.clone(),
                            location_desc: park.location_desc.clone(),
                            latitude: park.latitude,
                            longitude: park.longitude,
                        };
                        if let Err(e) = fetch_boundary(&pool, &client, &unfetched).await {
                            tracing::warn!(
                                "Park boundaries: {} refresh failed: {}",
                                park.reference,
                                e
                            );
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }
            Err(e) => {
                tracing::error!("Park boundaries: get_stale_boundaries failed: {}", e);
            }
        }

        let total = park_boundaries::count_boundaries(&pool).await.unwrap_or(0);
        tracing::info!("Park boundaries: {} total cached", total);

        tokio::time::sleep(std::time::Duration::from_secs(config.cycle_hours * 3600)).await;
    }
}

/// Fetch boundary for a single park from PAD-US.
async fn fetch_boundary(
    pool: &PgPool,
    client: &reqwest::Client,
    park: &UnfetchedPark,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state_name = park
        .location_desc
        .as_deref()
        .and_then(state_code_to_name);

    // Determine which service to query based on name patterns
    let is_federal = is_federal_park(&park.name);
    let service_url = if is_federal {
        PADUS_FEDERAL_URL
    } else {
        PADUS_STATE_URL
    };

    // Strategy 1: Name + state matching
    let search_name = normalize_park_name(&park.name);
    if let Some(state) = &state_name {
        if let Some(feature) = query_by_name(client, service_url, &search_name, state).await? {
            return save_feature(pool, park, &feature, "exact").await;
        }

        // Try the other service if first didn't match
        let alt_url = if is_federal {
            PADUS_STATE_URL
        } else {
            PADUS_FEDERAL_URL
        };
        if let Some(feature) = query_by_name(client, alt_url, &search_name, state).await? {
            return save_feature(pool, park, &feature, "exact").await;
        }
    }

    // Strategy 2: Spatial query (point-in-polygon)
    if let (Some(lat), Some(lon)) = (park.latitude, park.longitude) {
        if let Some(feature) = query_by_point(client, PADUS_FEDERAL_URL, lon, lat).await? {
            return save_feature(pool, park, &feature, "spatial").await;
        }
        if let Some(feature) = query_by_point(client, PADUS_STATE_URL, lon, lat).await? {
            return save_feature(pool, park, &feature, "spatial").await;
        }
    }

    tracing::debug!("Park boundaries: no match for {}", park.reference);
    Ok(())
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

    let resp: ArcGisResponse = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let features = resp.features.unwrap_or_default();
    // Prefer Designation feature class over Fee
    let best = features.into_iter().min_by_key(|f| {
        match f
            .attributes
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

    let resp: ArcGisResponse = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let features = resp.features.unwrap_or_default();
    let best = features.into_iter().min_by_key(|f| {
        match f
            .attributes
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
        None => return Ok(()),
    };

    let attrs = feature.attributes.as_ref();
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

    tracing::debug!(
        "Park boundaries: cached {} ({})",
        park.reference,
        match_quality
    );
    Ok(())
}

/// Simple URL encoding for the where clause.
fn urlencoded(s: &str) -> String {
    // Percent must be encoded first to avoid double-encoding
    s.replace('%', "%25")
        .replace(' ', "%20")
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

/// Check if a park name suggests federal management.
fn is_federal_park(name: &str) -> bool {
    let federal_keywords = [
        "National Park",
        "National Forest",
        "National Wildlife Refuge",
        "National Recreation Area",
        "National Monument",
        "National Seashore",
        "National Lakeshore",
        "National Grassland",
        "National Scenic Trail",
        "BLM",
    ];
    federal_keywords.iter().any(|kw| name.contains(kw))
}

/// Convert POTA state code (e.g. "US-ME") to full state name for PAD-US queries.
fn state_code_to_name(code: &str) -> Option<&'static str> {
    let state = code.strip_prefix("US-")?;
    match state {
        "AL" => Some("Alabama"),
        "AK" => Some("Alaska"),
        "AZ" => Some("Arizona"),
        "AR" => Some("Arkansas"),
        "CA" => Some("California"),
        "CO" => Some("Colorado"),
        "CT" => Some("Connecticut"),
        "DE" => Some("Delaware"),
        "FL" => Some("Florida"),
        "GA" => Some("Georgia"),
        "HI" => Some("Hawaii"),
        "ID" => Some("Idaho"),
        "IL" => Some("Illinois"),
        "IN" => Some("Indiana"),
        "IA" => Some("Iowa"),
        "KS" => Some("Kansas"),
        "KY" => Some("Kentucky"),
        "LA" => Some("Louisiana"),
        "ME" => Some("Maine"),
        "MD" => Some("Maryland"),
        "MA" => Some("Massachusetts"),
        "MI" => Some("Michigan"),
        "MN" => Some("Minnesota"),
        "MS" => Some("Mississippi"),
        "MO" => Some("Missouri"),
        "MT" => Some("Montana"),
        "NE" => Some("Nebraska"),
        "NV" => Some("Nevada"),
        "NH" => Some("New Hampshire"),
        "NJ" => Some("New Jersey"),
        "NM" => Some("New Mexico"),
        "NY" => Some("New York"),
        "NC" => Some("North Carolina"),
        "ND" => Some("North Dakota"),
        "OH" => Some("Ohio"),
        "OK" => Some("Oklahoma"),
        "OR" => Some("Oregon"),
        "PA" => Some("Pennsylvania"),
        "RI" => Some("Rhode Island"),
        "SC" => Some("South Carolina"),
        "SD" => Some("South Dakota"),
        "TN" => Some("Tennessee"),
        "TX" => Some("Texas"),
        "UT" => Some("Utah"),
        "VT" => Some("Vermont"),
        "VA" => Some("Virginia"),
        "WA" => Some("Washington"),
        "WV" => Some("West Virginia"),
        "WI" => Some("Wisconsin"),
        "WY" => Some("Wyoming"),
        "DC" => Some("District of Columbia"),
        "AS" => Some("American Samoa"),
        "GU" => Some("Guam"),
        "MP" => Some("Northern Mariana Islands"),
        "PR" => Some("Puerto Rico"),
        "VI" => Some("U.S. Virgin Islands"),
        _ => None,
    }
}
