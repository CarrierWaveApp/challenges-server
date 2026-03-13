pub mod historic_trails;
pub mod park_boundaries;
pub mod polish_park_boundaries;
pub mod pota;
pub mod pota_stats;
pub mod sota;
pub mod state_park_sources;

use sqlx::PgPool;

use crate::config::Config;
use crate::metrics as app_metrics;

/// Spawn all aggregator background tasks and the TTL cleanup task.
pub fn spawn_aggregators(pool: PgPool, config: &Config) {
    // TTL cleanup always runs
    let cleanup_pool = pool.clone();
    tokio::spawn(async move {
        ttl_cleanup_loop(cleanup_pool).await;
    });

    // Shared HTTP client for all aggregators
    let client = reqwest::Client::builder()
        .user_agent(format!(
            "{}/{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .expect("failed to build HTTP client");

    if config.pota_aggregator_enabled {
        let pota_pool = pool.clone();
        let pota_client = client.clone();
        tokio::spawn(async move {
            pota::poll_loop(pota_pool, pota_client).await;
        });
        tracing::info!("POTA aggregator started");
    }

    if config.sota_aggregator_enabled {
        let sota_pool = pool.clone();
        let sota_client = client.clone();
        tokio::spawn(async move {
            sota::poll_loop(sota_pool, sota_client).await;
        });
        tracing::info!("SOTA aggregator started");
    }
}

/// Spawn the historic trails aggregator.
pub fn spawn_historic_trails_aggregator(pool: PgPool, config: &Config) {
    let client = reqwest::Client::builder()
        .user_agent(format!(
            "{}/{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .expect("failed to build HTTP client");
    let trails_config = historic_trails::HistoricTrailsConfig {
        batch_size: config.historic_trails_batch_size,
        cycle_hours: config.historic_trails_cycle_hours,
        stale_days: config.historic_trails_stale_days,
        concurrency: config.historic_trails_concurrency,
    };
    tokio::spawn(async move {
        historic_trails::poll_loop(pool, client, trails_config).await;
    });
    tracing::info!("Historic trails aggregator started");
}

/// Spawn the park boundaries aggregator (requires POTA stats for park catalog).
pub fn spawn_park_boundaries_aggregator(pool: PgPool, config: &Config) {
    let client = reqwest::Client::builder()
        .user_agent(format!(
            "{}/{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .expect("failed to build HTTP client");
    let boundaries_config = park_boundaries::ParkBoundariesConfig {
        batch_size: config.park_boundaries_batch_size,
        cycle_hours: config.park_boundaries_cycle_hours,
        stale_days: config.park_boundaries_stale_days,
        concurrency: config.park_boundaries_concurrency,
    };
    tokio::spawn(async move {
        park_boundaries::poll_loop(pool, client, boundaries_config).await;
    });
    tracing::info!("Park boundaries aggregator started");
}

/// Spawn the Polish park boundaries aggregator (requires POTA stats for park catalog).
pub fn spawn_polish_park_boundaries_aggregator(pool: PgPool, config: &Config) {
    let client = reqwest::Client::builder()
        .user_agent(format!(
            "{}/{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .expect("failed to build HTTP client");
    let boundaries_config = polish_park_boundaries::PolishParkBoundariesConfig {
        batch_size: config.polish_park_boundaries_batch_size,
        cycle_hours: config.polish_park_boundaries_cycle_hours,
        stale_days: config.polish_park_boundaries_stale_days,
        concurrency: config.polish_park_boundaries_concurrency,
    };
    tokio::spawn(async move {
        polish_park_boundaries::poll_loop(pool, client, boundaries_config).await;
    });
    tracing::info!("Polish park boundaries aggregator started");
}

/// Spawn the POTA stats aggregator (independent of the spots system).
pub fn spawn_pota_stats_aggregator(pool: PgPool, config: &Config) {
    let client = reqwest::Client::builder()
        .user_agent(format!(
            "{}/{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .expect("failed to build HTTP client");
    let stats_config = pota_stats::PotaStatsConfig {
        concurrency: config.pota_stats_concurrency,
        batch_size: config.pota_stats_batch_size,
        cycle_hours: config.pota_stats_cycle_hours,
    };
    tokio::spawn(async move {
        pota_stats::poll_loop(pool, client, stats_config).await;
    });
    tracing::info!("POTA stats aggregator started");
}

/// Delete expired spots every 2 minutes.
async fn ttl_cleanup_loop(pool: PgPool) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(120));

    loop {
        interval.tick().await;
        match crate::db::delete_expired_spots(&pool).await {
            Ok(count) => {
                if count > 0 {
                    tracing::debug!("TTL cleanup: deleted {} expired spots", count);
                }
            }
            Err(e) => {
                tracing::error!("TTL cleanup error: {}", e);
                metrics::counter!(app_metrics::SYNC_ERRORS_TOTAL, "aggregator" => "ttl_cleanup")
                    .increment(1);
            }
        }
    }
}
