use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::performance_report::{
    AdminListPerformanceReportsQuery, CategoryBreakdown, CreatePerformanceReportRequest,
    PerformanceReportRow, PerformanceReportStats, VersionBreakdown,
};

/// Insert a new performance report.
pub async fn create_report(
    pool: &PgPool,
    callsign: &str,
    req: &CreatePerformanceReportRequest,
) -> Result<PerformanceReportRow, AppError> {
    let severity = req.severity.as_deref().unwrap_or("warning");

    let row = sqlx::query_as::<_, PerformanceReportRow>(
        r#"
        INSERT INTO performance_reports
            (callsign, category, duration_seconds, context, severity,
             app_version, build_number, device_model, os_version,
             diagnostic_payload, occurred_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING *
        "#,
    )
    .bind(callsign)
    .bind(&req.category)
    .bind(req.duration_seconds)
    .bind(&req.context)
    .bind(severity)
    .bind(&req.app_version)
    .bind(&req.build_number)
    .bind(&req.device_model)
    .bind(&req.os_version)
    .bind(&req.diagnostic_payload)
    .bind(req.occurred_at)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

/// List performance reports with filtering (admin).
pub async fn list_reports(
    pool: &PgPool,
    query: &AdminListPerformanceReportsQuery,
) -> Result<(Vec<PerformanceReportRow>, i64), AppError> {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let rows = sqlx::query_as::<_, PerformanceReportRow>(
        r#"
        SELECT *
        FROM performance_reports
        WHERE ($1::text IS NULL OR callsign = $1)
          AND ($2::text IS NULL OR category = $2)
          AND ($3::text IS NULL OR severity = $3)
          AND ($4::double precision IS NULL OR duration_seconds >= $4)
          AND ($5::text IS NULL OR app_version = $5)
          AND ($6::timestamptz IS NULL OR created_at >= $6)
        ORDER BY created_at DESC
        LIMIT $7 OFFSET $8
        "#,
    )
    .bind(&query.callsign)
    .bind(&query.category)
    .bind(&query.severity)
    .bind(query.min_duration)
    .bind(&query.app_version)
    .bind(query.since)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let (total,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM performance_reports
        WHERE ($1::text IS NULL OR callsign = $1)
          AND ($2::text IS NULL OR category = $2)
          AND ($3::text IS NULL OR severity = $3)
          AND ($4::double precision IS NULL OR duration_seconds >= $4)
          AND ($5::text IS NULL OR app_version = $5)
          AND ($6::timestamptz IS NULL OR created_at >= $6)
        "#,
    )
    .bind(&query.callsign)
    .bind(&query.category)
    .bind(&query.severity)
    .bind(query.min_duration)
    .bind(&query.app_version)
    .bind(query.since)
    .fetch_one(pool)
    .await?;

    Ok((rows, total))
}

/// Get a single performance report by ID.
pub async fn get_report(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<PerformanceReportRow>, AppError> {
    let row = sqlx::query_as::<_, PerformanceReportRow>(
        "SELECT * FROM performance_reports WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Get aggregate stats, optionally filtered by time range.
pub async fn get_stats(
    pool: &PgPool,
    since: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<PerformanceReportStats, AppError> {
    let stats = sqlx::query_as::<_, PerformanceReportStats>(
        r#"
        SELECT
            COUNT(*) AS total_reports,
            COUNT(DISTINCT callsign) AS unique_callsigns,
            AVG(duration_seconds) AS avg_duration_seconds,
            MAX(duration_seconds) AS max_duration_seconds
        FROM performance_reports
        WHERE ($1::timestamptz IS NULL OR created_at >= $1)
        "#,
    )
    .bind(since)
    .fetch_one(pool)
    .await?;

    Ok(stats)
}

/// Get per-category breakdown.
pub async fn get_category_breakdown(
    pool: &PgPool,
    since: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<Vec<CategoryBreakdown>, AppError> {
    let rows = sqlx::query_as::<_, CategoryBreakdown>(
        r#"
        SELECT category, COUNT(*) AS count, AVG(duration_seconds) AS avg_duration_seconds
        FROM performance_reports
        WHERE ($1::timestamptz IS NULL OR created_at >= $1)
        GROUP BY category
        ORDER BY count DESC
        "#,
    )
    .bind(since)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Get per-version breakdown.
pub async fn get_version_breakdown(
    pool: &PgPool,
    since: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<Vec<VersionBreakdown>, AppError> {
    let rows = sqlx::query_as::<_, VersionBreakdown>(
        r#"
        SELECT app_version, COUNT(*) AS count
        FROM performance_reports
        WHERE ($1::timestamptz IS NULL OR created_at >= $1)
        GROUP BY app_version
        ORDER BY count DESC
        "#,
    )
    .bind(since)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
