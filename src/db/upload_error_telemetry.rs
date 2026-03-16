use sqlx::PgPool;

use crate::error::AppError;
use crate::models::upload_error_telemetry::UploadErrorEntry;

/// Insert a batch of upload error telemetry entries for a callsign.
pub async fn insert_upload_errors(
    pool: &PgPool,
    callsign: &str,
    errors: &[UploadErrorEntry],
) -> Result<usize, AppError> {
    let mut count = 0;
    for entry in errors {
        sqlx::query(
            r#"
            INSERT INTO upload_error_telemetry
                (callsign, service, category, message_hash, affected_count, is_transient, app_version, os_version)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(callsign)
        .bind(&entry.service)
        .bind(&entry.category)
        .bind(&entry.message_hash)
        .bind(entry.affected_count)
        .bind(entry.is_transient)
        .bind(&entry.app_version)
        .bind(&entry.os_version)
        .execute(pool)
        .await?;
        count += 1;
    }
    Ok(count)
}
