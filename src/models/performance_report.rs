use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Database row for performance_reports table.
#[derive(Debug, sqlx::FromRow)]
pub struct PerformanceReportRow {
    pub id: Uuid,
    pub callsign: String,
    pub category: String,
    pub duration_seconds: Option<f64>,
    pub context: Option<String>,
    pub severity: String,
    pub app_version: Option<String>,
    pub build_number: Option<String>,
    pub device_model: Option<String>,
    pub os_version: Option<String>,
    pub diagnostic_payload: Option<serde_json::Value>,
    pub occurred_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// API request for submitting a performance report.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePerformanceReportRequest {
    pub category: String,
    pub duration_seconds: Option<f64>,
    pub context: Option<String>,
    pub severity: Option<String>,
    pub app_version: Option<String>,
    pub build_number: Option<String>,
    pub device_model: Option<String>,
    pub os_version: Option<String>,
    pub diagnostic_payload: Option<serde_json::Value>,
    pub occurred_at: DateTime<Utc>,
}

/// API response for a single performance report.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceReportResponse {
    pub id: Uuid,
    pub callsign: String,
    pub category: String,
    pub duration_seconds: Option<f64>,
    pub context: Option<String>,
    pub severity: String,
    pub app_version: Option<String>,
    pub build_number: Option<String>,
    pub device_model: Option<String>,
    pub os_version: Option<String>,
    pub diagnostic_payload: Option<serde_json::Value>,
    pub occurred_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl From<PerformanceReportRow> for PerformanceReportResponse {
    fn from(row: PerformanceReportRow) -> Self {
        Self {
            id: row.id,
            callsign: row.callsign,
            category: row.category,
            duration_seconds: row.duration_seconds,
            context: row.context,
            severity: row.severity,
            app_version: row.app_version,
            build_number: row.build_number,
            device_model: row.device_model,
            os_version: row.os_version,
            diagnostic_payload: row.diagnostic_payload,
            occurred_at: row.occurred_at,
            created_at: row.created_at,
        }
    }
}

/// Query params for admin listing performance reports.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminListPerformanceReportsQuery {
    pub callsign: Option<String>,
    pub category: Option<String>,
    pub severity: Option<String>,
    pub min_duration: Option<f64>,
    pub app_version: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Aggregate stats for admin dashboard.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceReportStats {
    pub total_reports: i64,
    pub unique_callsigns: i64,
    pub avg_duration_seconds: Option<f64>,
    pub max_duration_seconds: Option<f64>,
}

/// Per-category breakdown.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CategoryBreakdown {
    pub category: String,
    pub count: i64,
    pub avg_duration_seconds: Option<f64>,
}

/// Per-version breakdown.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct VersionBreakdown {
    pub app_version: Option<String>,
    pub count: i64,
}
