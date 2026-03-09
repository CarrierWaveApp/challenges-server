use axum::{Extension, Json};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::rbn::SpotStore;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rbn: Option<RbnHealth>,
}

#[derive(Serialize)]
pub struct RbnHealth {
    pub connected: bool,
    pub spots_in_store: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oldest_spot: Option<DateTime<Utc>>,
    pub spots_per_minute: f64,
}

pub async fn health_check(Extension(rbn_store): Extension<SpotStore>) -> Json<HealthResponse> {
    let (size, oldest) = rbn_store.health_info();
    let stats = rbn_store.stats(1);

    let rbn = if rbn_store.is_connected() || size > 0 {
        Some(RbnHealth {
            connected: rbn_store.is_connected(),
            spots_in_store: size,
            oldest_spot: oldest,
            spots_per_minute: stats.spots_per_minute,
        })
    } else {
        None
    };

    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        rbn,
    })
}
