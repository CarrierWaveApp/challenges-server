use axum::extract::{Query, State};
use axum::Extension;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::db;
use crate::error::AppError;
use crate::extractors::{Json, Path};
use crate::models::performance_report::{
    AdminListPerformanceReportsQuery, CreatePerformanceReportRequest, PerformanceReportResponse,
};
use sqlx::PgPool;

use super::DataResponse;

const VALID_CATEGORIES: &[&str] = &["hang", "slow_launch", "memory_warning", "crash_diagnostic", "other"];
const VALID_SEVERITIES: &[&str] = &["info", "warning", "critical"];

/// POST /v1/performance-reports
/// Submit a performance report (auth required).
pub async fn create_performance_report(
    State(pool): State<PgPool>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreatePerformanceReportRequest>,
) -> Result<(axum::http::StatusCode, Json<DataResponse<PerformanceReportResponse>>), AppError> {
    if !VALID_CATEGORIES.contains(&req.category.as_str()) {
        return Err(AppError::Validation {
            message: format!(
                "category must be one of: {}",
                VALID_CATEGORIES.join(", ")
            ),
        });
    }

    if let Some(ref severity) = req.severity {
        if !VALID_SEVERITIES.contains(&severity.as_str()) {
            return Err(AppError::Validation {
                message: format!(
                    "severity must be one of: {}",
                    VALID_SEVERITIES.join(", ")
                ),
            });
        }
    }

    if let Some(duration) = req.duration_seconds {
        if duration < 0.0 {
            return Err(AppError::Validation {
                message: "duration_seconds must be non-negative".to_string(),
            });
        }
    }

    let row = db::performance_reports::create_report(&pool, &auth.callsign, &req).await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(DataResponse {
            data: PerformanceReportResponse::from(row),
        }),
    ))
}

/// GET /v1/admin/performance-reports
/// List performance reports with filtering.
pub async fn list_performance_reports_admin(
    State(pool): State<PgPool>,
    Query(query): Query<AdminListPerformanceReportsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (rows, total) = db::performance_reports::list_reports(&pool, &query).await?;

    let reports: Vec<PerformanceReportResponse> =
        rows.into_iter().map(PerformanceReportResponse::from).collect();

    Ok(Json(serde_json::json!({
        "data": {
            "reports": reports,
            "total": total,
            "limit": query.limit.unwrap_or(50).min(100),
            "offset": query.offset.unwrap_or(0)
        }
    })))
}

/// GET /v1/admin/performance-reports/:id
/// Get a single performance report.
pub async fn get_performance_report_admin(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<DataResponse<PerformanceReportResponse>>, AppError> {
    let row = db::performance_reports::get_report(&pool, id)
        .await?
        .ok_or(AppError::Validation {
            message: "Performance report not found".to_string(),
        })?;

    Ok(Json(DataResponse {
        data: PerformanceReportResponse::from(row),
    }))
}

/// GET /v1/admin/performance-reports/stats
/// Get aggregate performance stats with category and version breakdowns.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsQuery {
    pub since: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn get_performance_stats_admin(
    State(pool): State<PgPool>,
    Query(query): Query<StatsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let stats = db::performance_reports::get_stats(&pool, query.since).await?;
    let categories = db::performance_reports::get_category_breakdown(&pool, query.since).await?;
    let versions = db::performance_reports::get_version_breakdown(&pool, query.since).await?;

    Ok(Json(serde_json::json!({
        "data": {
            "stats": stats,
            "categories": categories,
            "versions": versions
        }
    })))
}
