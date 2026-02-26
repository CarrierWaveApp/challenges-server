use sqlx::PgPool;

/// Poll SOTA spots from api2.sota.org.uk every 90 seconds.
/// Phase 2: implement actual polling logic.
pub async fn poll_loop(_pool: PgPool) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(90));

    loop {
        interval.tick().await;
        tracing::debug!("SOTA aggregator tick (not yet implemented)");
    }
}
