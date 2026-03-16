use serde::{Deserialize, Serialize};

/// Single error entry in the telemetry report
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UploadErrorEntry {
    pub service: String,
    pub category: String,
    pub message_hash: String,
    pub affected_count: i32,
    pub is_transient: bool,
    pub app_version: String,
    pub os_version: String,
}

/// Request body for POST /v1/telemetry/upload-errors
#[derive(Debug, Deserialize)]
pub struct ReportUploadErrorsRequest {
    pub errors: Vec<UploadErrorEntry>,
}

/// Response for the telemetry endpoint
#[derive(Debug, Serialize)]
pub struct ReportUploadErrorsResponse {
    pub accepted: usize,
}
