use metrics_exporter_prometheus::PrometheusHandle;

/// Metric name constants.
pub const GIS_FETCH_TOTAL: &str = "gis_fetch_total";
pub const GIS_FETCH_DURATION_SECONDS: &str = "gis_fetch_duration_seconds";
pub const GIS_BOUNDARIES_CACHED_TOTAL: &str = "gis_boundaries_cached_total";
pub const GIS_TRAILS_CACHED_TOTAL: &str = "gis_trails_cached_total";
pub const GIS_BATCH_DURATION_SECONDS: &str = "gis_batch_duration_seconds";

/// Install the Prometheus metrics exporter and return a handle for rendering.
pub fn install() -> PrometheusHandle {
    metrics_exporter_prometheus::PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install Prometheus recorder")
}
