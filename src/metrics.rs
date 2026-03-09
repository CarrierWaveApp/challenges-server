use std::sync::OnceLock;
use std::time::Instant;

use axum::{
    body::Body,
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Server start time for uptime calculation.
static START_TIME: OnceLock<Instant> = OnceLock::new();

// ---------------------------------------------------------------------------
// Metric name constants
// ---------------------------------------------------------------------------

// HTTP
pub const HTTP_REQUESTS_TOTAL: &str = "http_requests_total";
pub const HTTP_REQUEST_DURATION_SECONDS: &str = "http_request_duration_seconds";
pub const HTTP_REQUESTS_IN_FLIGHT: &str = "http_requests_in_flight";

// Auth
pub const AUTH_TOKEN_VALIDATIONS_TOTAL: &str = "auth_token_validations_total";
pub const AUTH_TOKENS_ISSUED_TOTAL: &str = "auth_tokens_issued_total";

// Database pool
pub const DB_POOL_CONNECTIONS_ACTIVE: &str = "db_pool_connections_active";
pub const DB_POOL_CONNECTIONS_IDLE: &str = "db_pool_connections_idle";
pub const DB_POOL_SIZE: &str = "db_pool_size";

// RBN
pub const RBN_CONNECTED: &str = "rbn_connected";
pub const RBN_SPOTS_INGESTED_TOTAL: &str = "rbn_spots_ingested_total";
pub const RBN_STORE_SIZE: &str = "rbn_store_size";
pub const RBN_PARSE_ERRORS_TOTAL: &str = "rbn_parse_errors_total";

// Aggregators
pub const AGGREGATOR_SYNC_DURATION_SECONDS: &str = "aggregator_sync_duration_seconds";
pub const AGGREGATOR_RECORDS_SYNCED_TOTAL: &str = "aggregator_records_synced_total";
pub const AGGREGATOR_ERRORS_TOTAL: &str = "aggregator_errors_total";

// Unique clients
pub const UNIQUE_CLIENTS_TOTAL: &str = "unique_clients_seen_total";

// Process
pub const PROCESS_UPTIME_SECONDS: &str = "process_uptime_seconds";

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

/// Install the Prometheus recorder and return the handle for rendering.
/// Must be called once at startup before any metrics are recorded.
pub fn init_metrics() -> PrometheusHandle {
    START_TIME.get_or_init(Instant::now);

    let recorder = PrometheusBuilder::new().build_recorder();
    let handle = recorder.handle();
    metrics::set_global_recorder(recorder).expect("failed to install metrics recorder");

    PROMETHEUS_HANDLE.get_or_init(|| handle).clone()
}

/// Render all metrics in Prometheus text exposition format.
pub fn render_metrics(handle: &PrometheusHandle) -> String {
    // Update uptime gauge before rendering
    if let Some(start) = START_TIME.get() {
        gauge!(PROCESS_UPTIME_SECONDS).set(start.elapsed().as_secs_f64());
    }
    handle.render()
}

// ---------------------------------------------------------------------------
// HTTP metrics middleware
// ---------------------------------------------------------------------------

/// Axum middleware that records request count, duration, and in-flight gauge.
pub async fn track_http_metrics(req: Request<Body>, next: Next) -> Response {
    let method = req.method().clone();
    let path = normalize_path(req.uri().path());

    gauge!(HTTP_REQUESTS_IN_FLIGHT, "method" => method.to_string(), "path" => path.clone())
        .increment(1.0);

    let start = Instant::now();
    let response = next.run(req).await;
    let elapsed = start.elapsed().as_secs_f64();

    let status = response.status().as_u16().to_string();

    counter!(HTTP_REQUESTS_TOTAL,
        "method" => method.to_string(),
        "path" => path.clone(),
        "status" => status.clone()
    )
    .increment(1);

    histogram!(HTTP_REQUEST_DURATION_SECONDS,
        "method" => method.to_string(),
        "path" => path.clone(),
        "status" => status
    )
    .record(elapsed);

    gauge!(HTTP_REQUESTS_IN_FLIGHT, "method" => method.to_string(), "path" => path)
        .decrement(1.0);

    response
}

/// Normalize a request path to collapse dynamic segments into placeholders.
/// This prevents high-cardinality labels from UUIDs and other path params.
fn normalize_path(path: &str) -> String {
    let segments: Vec<&str> = path.split('/').collect();
    let normalized: Vec<&str> = segments
        .iter()
        .map(|seg| {
            // Replace UUIDs
            if seg.len() == 36 && seg.chars().filter(|c| *c == '-').count() == 4 {
                ":id"
            // Replace numeric IDs
            } else if !seg.is_empty() && seg.chars().all(|c| c.is_ascii_digit()) {
                ":id"
            // Replace invite tokens (inv_ prefix)
            } else if seg.starts_with("inv_") {
                ":token"
            // Replace friend invite tokens (finv_ prefix)
            } else if seg.starts_with("finv_") {
                ":token"
            } else {
                seg
            }
        })
        .collect();
    normalized.join("/")
}

// ---------------------------------------------------------------------------
// Helpers for instrumentation call sites
// ---------------------------------------------------------------------------

/// Record a successful auth token validation.
pub fn record_auth_validation(outcome: &str) {
    counter!(AUTH_TOKEN_VALIDATIONS_TOTAL, "outcome" => outcome.to_string()).increment(1);
}

/// Record a new device token issuance.
pub fn record_token_issued() {
    counter!(AUTH_TOKENS_ISSUED_TOTAL).increment(1);
}

/// Record DB pool stats from sqlx.
pub fn record_db_pool_stats(pool: &sqlx::PgPool) {
    let size = pool.size() as f64;
    let idle = pool.num_idle() as f64;
    let active = size - idle;

    gauge!(DB_POOL_SIZE).set(size);
    gauge!(DB_POOL_CONNECTIONS_IDLE).set(idle);
    gauge!(DB_POOL_CONNECTIONS_ACTIVE).set(active);
}

/// Record RBN connection state.
pub fn record_rbn_connected(connected: bool) {
    gauge!(RBN_CONNECTED).set(if connected { 1.0 } else { 0.0 });
}

/// Record spots ingested into the RBN store.
pub fn record_rbn_spots_ingested(count: u64) {
    counter!(RBN_SPOTS_INGESTED_TOTAL).increment(count);
}

/// Record current RBN store size.
pub fn record_rbn_store_size(size: usize) {
    gauge!(RBN_STORE_SIZE).set(size as f64);
}

/// Record an RBN parse error.
pub fn record_rbn_parse_error() {
    counter!(RBN_PARSE_ERRORS_TOTAL).increment(1);
}

/// Record aggregator sync duration.
pub fn record_aggregator_sync_duration(aggregator: &str, seconds: f64) {
    histogram!(AGGREGATOR_SYNC_DURATION_SECONDS, "aggregator" => aggregator.to_string())
        .record(seconds);
}

/// Record records synced by an aggregator.
pub fn record_aggregator_records_synced(aggregator: &str, count: u64) {
    counter!(AGGREGATOR_RECORDS_SYNCED_TOTAL, "aggregator" => aggregator.to_string())
        .increment(count);
}

/// Record an aggregator error.
pub fn record_aggregator_error(aggregator: &str) {
    counter!(AGGREGATOR_ERRORS_TOTAL, "aggregator" => aggregator.to_string()).increment(1);
}

/// Record a unique client (by IP or token).
pub fn record_unique_client() {
    counter!(UNIQUE_CLIENTS_TOTAL).increment(1);
}

// ---------------------------------------------------------------------------
// Metrics endpoint handler
// ---------------------------------------------------------------------------

/// GET /metrics — Prometheus text exposition format.
pub async fn metrics_handler(
    axum::Extension(handle): axum::Extension<PrometheusHandle>,
) -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        render_metrics(&handle),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_uuid() {
        assert_eq!(
            normalize_path("/v1/challenges/550e8400-e29b-41d4-a716-446655440000/progress"),
            "/v1/challenges/:id/progress"
        );
    }

    #[test]
    fn test_normalize_path_numeric() {
        assert_eq!(normalize_path("/v1/items/42"), "/v1/items/:id");
    }

    #[test]
    fn test_normalize_path_no_change() {
        assert_eq!(
            normalize_path("/v1/challenges"),
            "/v1/challenges"
        );
    }

    #[test]
    fn test_normalize_path_invite_token() {
        assert_eq!(
            normalize_path("/v1/admin/invites/inv_abc123"),
            "/v1/admin/invites/:token"
        );
    }

    #[test]
    fn test_normalize_path_named_segment() {
        assert_eq!(
            normalize_path("/v1/programs/pota"),
            "/v1/programs/pota"
        );
    }
}
