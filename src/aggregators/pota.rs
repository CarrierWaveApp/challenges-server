use sqlx::PgPool;

/// Poll POTA activator spots from api.pota.app every 60 seconds.
/// Phase 2: implement actual polling logic.
pub async fn poll_loop(_pool: PgPool) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

    loop {
        interval.tick().await;
        tracing::debug!("POTA aggregator tick (not yet implemented)");
    }
}
