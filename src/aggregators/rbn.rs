use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use sqlx::PgPool;

use crate::db::upsert_aggregated_spot;
use crate::models::spot::{AggregatedSpot, SpotSource};

const RBN_SPOTS_URL: &str = "https://www.vailrerbn.com/api/v1/spots?limit=500";

/// Wrapper for the RBN API response.
#[derive(Debug, Deserialize)]
struct RbnResponse {
    spots: Vec<RbnSpot>,
}

/// Upstream JSON shape from the Vail ReRBN spots endpoint.
#[derive(Debug, Deserialize)]
struct RbnSpot {
    id: i64,
    callsign: String,
    frequency: f64,
    mode: String,
    timestamp: DateTime<Utc>,
    #[serde(default)]
    snr: Option<i16>,
    #[serde(default)]
    spotter: Option<String>,
    #[serde(default)]
    speed: Option<i16>,
}

/// Poll RBN spots every 30 seconds.
pub async fn poll_loop(pool: PgPool, client: reqwest::Client) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));

    loop {
        interval.tick().await;
        if let Err(e) = fetch_and_upsert(&pool, &client).await {
            tracing::error!("RBN aggregator error: {}", e);
        }
    }
}

async fn fetch_and_upsert(
    pool: &PgPool,
    client: &reqwest::Client,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resp: RbnResponse = client
        .get(RBN_SPOTS_URL)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    tracing::debug!("RBN: fetched {} spots", resp.spots.len());

    let mut upserted = 0u32;
    for spot in &resp.spots {
        let agg = map_spot(spot);
        match upsert_aggregated_spot(pool, &agg).await {
            Ok(_) => upserted += 1,
            Err(e) => tracing::warn!("RBN upsert error for {}: {}", spot.callsign, e),
        }
    }

    tracing::debug!("RBN: upserted {}/{} spots", upserted, resp.spots.len());
    Ok(())
}

fn map_spot(spot: &RbnSpot) -> AggregatedSpot {
    AggregatedSpot {
        callsign: spot.callsign.clone(),
        program_slug: None,
        source: SpotSource::Rbn,
        external_id: spot.id.to_string(),
        frequency_khz: spot.frequency,
        mode: spot.mode.clone(),
        reference: None,
        reference_name: None,
        spotter: spot.spotter.clone(),
        spotter_grid: None,
        location_desc: None,
        country_code: None,
        state_abbr: None,
        comments: None,
        snr: spot.snr,
        wpm: spot.speed,
        spotted_at: spot.timestamp,
        expires_at: spot.timestamp + Duration::minutes(10),
    }
}
