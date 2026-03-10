use std::io::Cursor;
use std::sync::Arc;

use chrono::NaiveDate;
use sqlx::PgPool;
use tokio::sync::Semaphore;

use crate::db::pota_stats;
use crate::models::pota_stats::{
    PotaApiActivation, PotaApiLeaderboard, PotaApiStats, PotaCsvPark,
};

const ALL_PARKS_CSV_URL: &str = "https://pota.app/all_parks_ext.csv";
const POTA_API_BASE: &str = "https://api.pota.app";

/// Configuration for the POTA stats aggregator.
pub struct PotaStatsConfig {
    pub concurrency: usize,
    pub batch_size: i64,
    pub cycle_hours: u64,
}

impl Default for PotaStatsConfig {
    fn default() -> Self {
        Self {
            concurrency: 3,
            batch_size: 50,
            cycle_hours: 24,
        }
    }
}

/// Main poll loop — runs forever, syncing park catalog then fetching stats in batches.
pub async fn poll_loop(pool: PgPool, client: reqwest::Client, config: PotaStatsConfig) {
    // Phase 1: Initial catalog sync
    loop {
        match sync_park_catalog(&pool, &client).await {
            Ok(count) => {
                tracing::info!("POTA stats: synced {} parks from catalog", count);
                break;
            }
            Err(e) => {
                tracing::error!("POTA stats: catalog sync failed: {}, retrying in 60s", e);
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        }
    }

    let semaphore = Arc::new(Semaphore::new(config.concurrency));

    // Phase 2: Continuous batch fetching
    loop {
        let total_parks = match pota_stats::count_parks(&pool).await {
            Ok(n) => n.max(1),
            Err(e) => {
                tracing::error!("POTA stats: count_parks failed: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                continue;
            }
        };

        let unfetched = pota_stats::count_unfetched_parks(&pool).await.unwrap_or(0);
        let is_initial = unfetched as f64 > total_parks as f64 * 0.5;

        // Calculate sleep between batches
        let total_batches = (total_parks as f64 / config.batch_size as f64).ceil() as u64;
        let sleep_secs = if is_initial {
            // Target ~1 hour for initial population
            3600 / total_batches.max(1)
        } else {
            (config.cycle_hours * 3600) / total_batches.max(1)
        };

        let stalest = match pota_stats::get_stalest_parks(&pool, config.batch_size).await {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!("POTA stats: get_stalest_parks failed: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                continue;
            }
        };

        if stalest.is_empty() {
            tracing::debug!("POTA stats: no parks to fetch, sleeping");
            tokio::time::sleep(std::time::Duration::from_secs(sleep_secs)).await;
            continue;
        }

        let mut succeeded = 0u32;
        let mut failed = 0u32;
        let batch_len = stalest.len();

        let mut handles = Vec::with_capacity(batch_len);

        for stale in stalest {
            let pool = pool.clone();
            let client = client.clone();
            let semaphore = semaphore.clone();

            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                fetch_park_data(&pool, &client, &stale.park_reference).await
            });
            handles.push(handle);
        }

        for handle in handles {
            match handle.await {
                Ok(Ok(())) => succeeded += 1,
                Ok(Err(e)) => {
                    tracing::warn!("POTA stats: park fetch error: {}", e);
                    failed += 1;
                }
                Err(e) => {
                    tracing::error!("POTA stats: task join error: {}", e);
                    failed += 1;
                }
            }
        }

        tracing::info!(
            "POTA stats: batch {}/{} succeeded, {} failed ({}% unfetched)",
            succeeded,
            batch_len,
            failed,
            if total_parks > 0 {
                (unfetched * 100) / total_parks
            } else {
                0
            }
        );

        // Re-sync catalog daily (check if we should)
        // We do this every cycle_hours by running it once at the top
        if !is_initial {
            // Periodic catalog re-sync every cycle
            if let Err(e) = sync_park_catalog(&pool, &client).await {
                tracing::warn!("POTA stats: periodic catalog sync failed: {}", e);
            }
            // Reset consecutive error counters so previously-failing parks
            // get retried next cycle (they may have been fixed upstream)
            match pota_stats::reset_consecutive_errors(&pool).await {
                Ok(n) if n > 0 => {
                    tracing::info!("POTA stats: reset error counters for {} parks", n);
                }
                Err(e) => {
                    tracing::warn!("POTA stats: failed to reset error counters: {}", e);
                }
                _ => {}
            }
        }

        // Phase 3: sleep between batches
        tokio::time::sleep(std::time::Duration::from_secs(sleep_secs)).await;
    }
}

/// Fetch and parse the all_parks_ext.csv, upserting US parks.
async fn sync_park_catalog(
    pool: &PgPool,
    client: &reqwest::Client,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let csv_bytes = client
        .get(ALL_PARKS_CSV_URL)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    let mut reader = csv::Reader::from_reader(Cursor::new(&csv_bytes));
    let mut count = 0usize;

    for result in reader.deserialize::<PotaCsvPark>() {
        let park = match result {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("POTA stats: CSV parse error: {}", e);
                continue;
            }
        };

        // Only include parks for countries with boundary data sources
        if !park.reference.starts_with("US-")
            && !park.reference.starts_with("G-")
            && !park.reference.starts_with("GM-")
            && !park.reference.starts_with("GW-")
            && !park.reference.starts_with("GI-")
            && !park.reference.starts_with("I-")
            && !park.reference.starts_with("SP-")
        {
            continue;
        }

        let active = park.active == "1";

        // Use locationDesc directly as the state key (e.g., "US-CA")
        let state = park.location_desc.clone();

        pota_stats::upsert_park(
            pool,
            &park.reference,
            &park.name,
            park.location_desc.as_deref(),
            state.as_deref(),
            park.lat,
            park.lon,
            park.grid.as_deref(),
            active,
        )
        .await?;

        pota_stats::ensure_fetch_status(pool, &park.reference).await?;

        count += 1;
    }

    Ok(count)
}

/// Fetch all data for a single park: stats, activations, and leaderboard.
async fn fetch_park_data(
    pool: &PgPool,
    client: &reqwest::Client,
    park_reference: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Get the park's state for denormalization
    let park = pota_stats::get_park_detail(pool, park_reference).await?;
    let state = park.as_ref().and_then(|p| p.state.clone());

    // Fetch park stats
    match fetch_park_stats(client, park_reference).await {
        Ok(stats) => {
            pota_stats::update_park_stats(
                pool,
                park_reference,
                stats.attempts,
                stats.activations,
                stats.contacts,
            )
            .await?;
        }
        Err(e) => {
            tracing::warn!(
                "POTA stats: /park/stats/{} failed: {}",
                park_reference,
                e
            );
        }
    }

    // Fetch activations
    let activations_ok = match fetch_park_activations(client, park_reference).await {
        Ok(activations) => {
            for act in &activations {
                let qso_date = match NaiveDate::parse_from_str(&act.qso_date, "%Y%m%d") {
                    Ok(d) => d,
                    Err(e) => {
                        tracing::warn!(
                            "POTA stats: bad date '{}' for {}/{}: {}",
                            act.qso_date,
                            park_reference,
                            act.active_callsign,
                            e
                        );
                        continue;
                    }
                };

                if let Err(e) = pota_stats::upsert_activation(
                    pool,
                    park_reference,
                    &act.active_callsign,
                    qso_date,
                    act.total_qsos,
                    act.qsos_cw,
                    act.qsos_data,
                    act.qsos_phone,
                    state.as_deref(),
                )
                .await
                {
                    tracing::warn!(
                        "POTA stats: upsert activation {}/{} failed: {}",
                        park_reference,
                        act.active_callsign,
                        e
                    );
                }
            }
            true
        }
        Err(e) => {
            let err_msg = format!("activations fetch failed: {}", e);
            tracing::warn!("POTA stats: /park/activations/{}: {}", park_reference, err_msg);
            pota_stats::record_fetch_error(pool, park_reference, &err_msg).await?;
            false
        }
    };

    // Fetch leaderboard (hunter QSOs)
    let leaderboard_ok = match fetch_park_leaderboard(client, park_reference).await {
        Ok(leaderboard) => {
            for hunter in &leaderboard.hunter_qsos {
                if let Err(e) = pota_stats::upsert_hunter_qsos(
                    pool,
                    park_reference,
                    &hunter.callsign,
                    hunter.count,
                    state.as_deref(),
                )
                .await
                {
                    tracing::warn!(
                        "POTA stats: upsert hunter {}/{} failed: {}",
                        park_reference,
                        hunter.callsign,
                        e
                    );
                }
            }
            true
        }
        Err(e) => {
            let err_msg = format!("leaderboard fetch failed: {}", e);
            tracing::warn!("POTA stats: /park/leaderboard/{}: {}", park_reference, err_msg);
            if activations_ok {
                // Only record error if activations succeeded (otherwise already recorded)
                pota_stats::record_fetch_error(pool, park_reference, &err_msg).await?;
            }
            false
        }
    };

    // Update fetch status
    if activations_ok || leaderboard_ok {
        pota_stats::update_fetch_status(pool, park_reference, activations_ok, leaderboard_ok)
            .await?;
    }

    Ok(())
}

async fn fetch_park_stats(
    client: &reqwest::Client,
    park_reference: &str,
) -> Result<PotaApiStats, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/park/stats/{}", POTA_API_BASE, park_reference);
    let stats: PotaApiStats = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(stats)
}

async fn fetch_park_activations(
    client: &reqwest::Client,
    park_reference: &str,
) -> Result<Vec<PotaApiActivation>, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!(
        "{}/park/activations/{}?count=all",
        POTA_API_BASE, park_reference
    );
    let activations: Vec<PotaApiActivation> = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(activations)
}

async fn fetch_park_leaderboard(
    client: &reqwest::Client,
    park_reference: &str,
) -> Result<PotaApiLeaderboard, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!(
        "{}/park/leaderboard/{}?count=all",
        POTA_API_BASE, park_reference
    );
    let leaderboard: PotaApiLeaderboard = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(leaderboard)
}
