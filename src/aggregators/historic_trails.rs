use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::Semaphore;

use crate::db::historic_trails::{self, UnfetchedTrail};
use crate::metrics as app_metrics;
use crate::models::historic_trail::{NpsTrailFeature, NpsTrailResponse};

use super::park_boundaries::merge_geojson_geometries;

const NPS_TRAILS_URL: &str = "https://services.arcgis.com/P3ePLMYs2RVChkJx/arcgis/rest/services/National_Trails_System/FeatureServer/0";

/// Configuration for the historic trails aggregator.
pub struct HistoricTrailsConfig {
    pub batch_size: i64,
    pub cycle_hours: u64,
    pub stale_days: i64,
    pub concurrency: usize,
}

impl Default for HistoricTrailsConfig {
    fn default() -> Self {
        Self {
            batch_size: 20,
            cycle_hours: 168, // weekly — only 19 trails
            stale_days: 180,
            concurrency: 5,
        }
    }
}

enum FetchResult {
    Cached(String),
    NoMatch,
    Error(String),
}

/// Main poll loop — fetches geometries for unmatched trails, then re-checks stale ones.
pub async fn poll_loop(pool: PgPool, client: reqwest::Client, config: HistoricTrailsConfig) {
    // Wait for migrations and initial setup
    tracing::info!("Historic trails: waiting 60s before first cycle");
    tokio::time::sleep(std::time::Duration::from_secs(60)).await;

    let semaphore = Arc::new(Semaphore::new(config.concurrency));

    loop {
        let batch_start = std::time::Instant::now();
        let total_cached = historic_trails::count_trails(&pool).await.unwrap_or(0);

        // Phase 1: Fetch geometries for trails that don't have one yet
        match historic_trails::get_unfetched_trails(&pool, config.batch_size).await {
            Ok(trails) => {
                if trails.is_empty() {
                    tracing::info!(
                        "Historic trails: all trails fetched ({} cached)",
                        total_cached
                    );
                } else {
                    tracing::info!(
                        "Historic trails: fetching {} unfetched trails ({} already cached)",
                        trails.len(),
                        total_cached
                    );

                    let (cached, no_match, errors) =
                        fetch_batch(&pool, &client, &semaphore, trails).await;

                    let new_total = historic_trails::count_trails(&pool).await.unwrap_or(0);
                    tracing::info!(
                        "Historic trails: batch done — {} cached, {} no match, {} errors ({} total cached)",
                        cached, no_match, errors, new_total
                    );
                }
            }
            Err(e) => {
                tracing::error!("Historic trails: get_unfetched_trails failed: {}", e);
            }
        }

        // Phase 2: Re-fetch stale trail geometries
        match historic_trails::get_stale_trails(&pool, config.stale_days, config.batch_size).await {
            Ok(stale) => {
                if !stale.is_empty() {
                    tracing::info!(
                        "Historic trails: refreshing {} stale trails",
                        stale.len()
                    );
                    let unfetched: Vec<UnfetchedTrail> = stale
                        .into_iter()
                        .map(|trail| UnfetchedTrail {
                            reference: trail.reference,
                            name: trail.name,
                            location_desc: trail.location_desc,
                            managing_agency: trail.managing_agency,
                        })
                        .collect();

                    fetch_batch(&pool, &client, &semaphore, unfetched).await;
                }
            }
            Err(e) => {
                tracing::error!("Historic trails: get_stale_trails failed: {}", e);
            }
        }

        // Record batch metrics
        let new_total = historic_trails::count_trails(&pool).await.unwrap_or(0);
        metrics::gauge!(app_metrics::GIS_TRAILS_CACHED_TOTAL).set(new_total as f64);
        metrics::histogram!(app_metrics::GIS_BATCH_DURATION_SECONDS, "aggregator" => "historic_trails")
            .record(batch_start.elapsed().as_secs_f64());

        tracing::info!(
            "Historic trails: sleeping {}h until next cycle",
            config.cycle_hours
        );
        tokio::time::sleep(std::time::Duration::from_secs(config.cycle_hours * 3600)).await;
    }
}

/// Fetch a batch of trails concurrently using the semaphore for rate limiting.
async fn fetch_batch(
    pool: &PgPool,
    client: &reqwest::Client,
    semaphore: &Arc<Semaphore>,
    trails: Vec<UnfetchedTrail>,
) -> (u32, u32, u32) {
    let mut handles = Vec::with_capacity(trails.len());

    for trail in trails {
        let pool = pool.clone();
        let client = client.clone();
        let semaphore = semaphore.clone();

        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            let start = std::time::Instant::now();
            let result = fetch_trail(&pool, &client, &trail).await;
            let result_label = match &result {
                FetchResult::Cached(_) => "cached",
                FetchResult::NoMatch => "no_match",
                FetchResult::Error(_) => "error",
            };
            metrics::counter!(app_metrics::GIS_FETCH_TOTAL, "source" => "nps_trails", "result" => result_label)
                .increment(1);
            metrics::histogram!(app_metrics::GIS_FETCH_DURATION_SECONDS, "source" => "nps_trails")
                .record(start.elapsed().as_secs_f64());
            (trail.reference, trail.name, result)
        });
        handles.push(handle);
    }

    let mut cached = 0u32;
    let mut no_match = 0u32;
    let mut errors = 0u32;

    for handle in handles {
        match handle.await {
            Ok((reference, name, FetchResult::Cached(quality))) => {
                tracing::info!(
                    "Historic trails: {} '{}' -> cached ({})",
                    reference,
                    name,
                    quality
                );
                cached += 1;
            }
            Ok((reference, name, FetchResult::NoMatch)) => {
                tracing::info!(
                    "Historic trails: {} '{}' -> no match",
                    reference,
                    name
                );
                no_match += 1;
            }
            Ok((reference, name, FetchResult::Error(e))) => {
                tracing::warn!(
                    "Historic trails: {} '{}' -> error: {}",
                    reference,
                    name,
                    e
                );
                errors += 1;
            }
            Err(e) => {
                tracing::error!("Historic trails: task join error: {}", e);
                errors += 1;
            }
        }
    }

    (cached, no_match, errors)
}

async fn fetch_trail(
    pool: &PgPool,
    client: &reqwest::Client,
    trail: &UnfetchedTrail,
) -> FetchResult {
    match fetch_trail_inner(pool, client, trail).await {
        Ok(Some(quality)) => FetchResult::Cached(quality),
        Ok(None) => FetchResult::NoMatch,
        Err(e) => FetchResult::Error(e.to_string()),
    }
}

async fn fetch_trail_inner(
    pool: &PgPool,
    client: &reqwest::Client,
    trail: &UnfetchedTrail,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    // Strategy: query NPS National Trails by name
    let search_name = normalize_trail_name(&trail.name);

    match query_by_name(client, NPS_TRAILS_URL, &search_name).await {
        Ok(Some(feature)) => {
            save_feature(pool, trail, &feature, "exact").await?;
            return Ok(Some("exact".to_string()));
        }
        Ok(None) => {}
        Err(e) => {
            tracing::warn!(
                "Historic trails: {} name query failed: {}",
                trail.reference,
                e
            );
        }
    }

    // Fallback: broader search with shorter name
    let short_name = search_name
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ");
    if short_name != search_name && !short_name.is_empty() {
        match query_by_name(client, NPS_TRAILS_URL, &short_name).await {
            Ok(Some(feature)) => {
                save_feature(pool, trail, &feature, "fuzzy").await?;
                return Ok(Some("fuzzy".to_string()));
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(
                    "Historic trails: {} short-name query failed: {}",
                    trail.reference,
                    e
                );
            }
        }
    }

    Ok(None)
}

/// Query NPS Trails FeatureServer by trail name — merges all features.
async fn query_by_name(
    client: &reqwest::Client,
    service_url: &str,
    search_name: &str,
) -> Result<Option<NpsTrailFeature>, Box<dyn std::error::Error + Send + Sync>> {
    let escaped_name = search_name.replace('\'', "''");
    let where_clause = format!("Trail_Name LIKE '%{}%'", escaped_name);

    let url = format!(
        "{}/query?where={}&outFields=Trail_Name,Mang_Agency,Designation,Length_MI,State&f=geojson&outSR=4326",
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

    let resp: NpsTrailResponse = match serde_json::from_str(&resp_text) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(
                "Historic trails: ArcGIS response parse error: {} (first 200 chars: {})",
                e,
                &resp_text[..resp_text.len().min(200)]
            );
            return Ok(None);
        }
    };

    let features = resp.features.unwrap_or_default();
    Ok(merge_trail_features(features))
}

/// Merge all trail features into a single feature with merged geometry.
/// A trail can have multiple line segments returned as separate features.
fn merge_trail_features(features: Vec<NpsTrailFeature>) -> Option<NpsTrailFeature> {
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

    let merged = merge_geojson_geometries(geometries);
    let mut result = features.into_iter().next().unwrap();
    result.geometry = Some(merged);
    Some(result)
}

/// Save a trail feature to the database.
async fn save_feature(
    pool: &PgPool,
    trail: &UnfetchedTrail,
    feature: &NpsTrailFeature,
    match_quality: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let geojson = match &feature.geometry {
        Some(g) => serde_json::to_string(g)?,
        None => {
            tracing::warn!(
                "Historic trails: {} matched but feature has no geometry",
                trail.reference
            );
            return Ok(());
        }
    };

    let attrs = feature.properties.as_ref();
    let designation = attrs.and_then(|a| a.designation.as_deref());
    let managing_agency = attrs.and_then(|a| a.managing_agency.as_deref());
    let length_miles = attrs.and_then(|a| a.length_miles);
    let state = attrs.and_then(|a| a.state.as_deref());

    historic_trails::upsert_trail(
        pool,
        &trail.reference,
        &trail.name,
        designation,
        managing_agency,
        length_miles,
        state,
        match_quality,
        &geojson,
        "nps_trails",
    )
    .await?;

    Ok(())
}

/// URL-encode a string for use in ArcGIS REST API query parameters.
fn urlencoded(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('\'', "%27")
        .replace('=', "%3D")
        .replace('&', "%26")
}

/// Normalize a trail name for search by stripping the "National Historic Trail" suffix.
fn normalize_trail_name(name: &str) -> String {
    let suffixes = [
        " National Historic Trail",
        " National Scenic Trail",
        " National Recreation Trail",
        " National Heritage Area Trail",
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
