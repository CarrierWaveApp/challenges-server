use chrono::{Duration, NaiveDateTime, Utc};
use serde::Deserialize;
use sqlx::PgPool;

use crate::db::upsert_aggregated_spot;
use crate::models::spot::{AggregatedSpot, SpotSource};

const POTA_SPOTS_URL: &str = "https://api.pota.app/spot/activator";

/// Upstream JSON shape from the POTA activator spots endpoint.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PotaSpot {
    spot_id: i64,
    activator: String,
    frequency: String,
    mode: String,
    reference: String,
    #[serde(default)]
    park_name: Option<String>,
    spot_time: String,
    #[serde(default)]
    spotter: Option<String>,
    #[serde(default)]
    comments: Option<String>,
    #[serde(default)]
    location_desc: Option<String>,
    /// Seconds until the spot expires. Absent or 0 → use 30 min default.
    #[serde(default)]
    expire: Option<i64>,
}

/// Poll POTA activator spots every 60 seconds.
pub async fn poll_loop(pool: PgPool, client: reqwest::Client) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

    loop {
        interval.tick().await;
        if let Err(e) = fetch_and_upsert(&pool, &client).await {
            tracing::error!("POTA aggregator error: {}", e);
        }
    }
}

async fn fetch_and_upsert(
    pool: &PgPool,
    client: &reqwest::Client,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let spots: Vec<PotaSpot> = client
        .get(POTA_SPOTS_URL)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    tracing::debug!("POTA: fetched {} spots", spots.len());

    let mut upserted = 0u32;
    for spot in &spots {
        match map_spot(spot) {
            Ok(agg) => match upsert_aggregated_spot(pool, &agg).await {
                Ok(_) => upserted += 1,
                Err(e) => tracing::warn!("POTA upsert error for {}: {}", spot.activator, e),
            },
            Err(e) => {
                tracing::warn!("POTA parse error spotId={}: {}", spot.spot_id, e);
            }
        }
    }

    tracing::debug!("POTA: upserted {}/{} spots", upserted, spots.len());
    Ok(())
}

fn map_spot(spot: &PotaSpot) -> Result<AggregatedSpot, Box<dyn std::error::Error + Send + Sync>> {
    let frequency_khz: f64 = spot.frequency.parse()?;

    // spotTime is UTC but has no Z suffix
    let spotted_at = NaiveDateTime::parse_from_str(&spot.spot_time, "%Y-%m-%dT%H:%M:%S")
        .map(|naive| naive.and_utc())?;

    // expire = seconds remaining; fallback 30 min
    let expires_at = match spot.expire {
        Some(secs) if secs > 0 => Utc::now() + Duration::seconds(secs),
        _ => Utc::now() + Duration::minutes(30),
    };

    // Split locationDesc (e.g. "US-WY") into country / state
    let (country_code, state_abbr) = spot
        .location_desc
        .as_deref()
        .map(|desc| {
            let mut parts = desc.splitn(2, '-');
            let country = parts.next().map(str::to_string);
            let state = parts.next().map(str::to_string);
            (country, state)
        })
        .unwrap_or((None, None));

    Ok(AggregatedSpot {
        callsign: spot.activator.clone(),
        program_slug: Some("pota".to_string()),
        source: SpotSource::Pota,
        external_id: spot.spot_id.to_string(),
        frequency_khz,
        mode: spot.mode.clone(),
        reference: Some(spot.reference.clone()),
        reference_name: spot.park_name.clone(),
        spotter: spot.spotter.clone(),
        spotter_grid: None,
        location_desc: spot.location_desc.clone(),
        country_code,
        state_abbr,
        comments: spot.comments.clone(),
        snr: None,
        wpm: None,
        spotted_at,
        expires_at,
    })
}
