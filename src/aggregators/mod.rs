pub mod pota;
pub mod rbn;
pub mod sota;

use sqlx::PgPool;

use crate::config::Config;

/// Spawn all aggregator background tasks and the TTL cleanup task.
pub fn spawn_aggregators(pool: PgPool, config: &Config) {
    // TTL cleanup always runs
    let cleanup_pool = pool.clone();
    tokio::spawn(async move {
        ttl_cleanup_loop(cleanup_pool).await;
    });

    if config.pota_aggregator_enabled {
        let pota_pool = pool.clone();
        tokio::spawn(async move {
            pota::poll_loop(pota_pool).await;
        });
        tracing::info!("POTA aggregator started");
    }

    if config.rbn_aggregator_enabled {
        let rbn_pool = pool.clone();
        tokio::spawn(async move {
            rbn::poll_loop(rbn_pool).await;
        });
        tracing::info!("RBN aggregator started");
    }

    if config.sota_aggregator_enabled {
        let sota_pool = pool.clone();
        tokio::spawn(async move {
            sota::poll_loop(sota_pool).await;
        });
        tracing::info!("SOTA aggregator started");
    }
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
            }
        }
    }
}
