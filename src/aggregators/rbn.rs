use sqlx::PgPool;

/// Poll RBN spots from vailrerbn.com every 30 seconds.
/// Phase 2: implement actual polling logic.
pub async fn poll_loop(_pool: PgPool) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));

    loop {
        interval.tick().await;
        tracing::debug!("RBN aggregator tick (not yet implemented)");
    }
}
