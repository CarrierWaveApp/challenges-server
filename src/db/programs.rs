use sqlx::PgPool;

use crate::error::AppError;
use crate::models::program::ProgramRow;

/// List all active programs ordered by sort_order.
pub async fn list_programs(pool: &PgPool) -> Result<Vec<ProgramRow>, AppError> {
    let rows = sqlx::query_as::<_, ProgramRow>(
        r#"
        SELECT slug, name, short_name, icon, icon_url, website, server_base_url,
               reference_label, reference_format, reference_example,
               multi_ref_allowed, activation_threshold, supports_rove, capabilities,
               adif_my_sig, adif_my_sig_info, adif_sig_field, adif_sig_info_field,
               data_entry_label, data_entry_placeholder, data_entry_format,
               sort_order, is_active, created_at, updated_at
        FROM programs
        WHERE is_active = true
        ORDER BY sort_order
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Get a single active program by slug.
pub async fn get_program(pool: &PgPool, slug: &str) -> Result<Option<ProgramRow>, AppError> {
    let row = sqlx::query_as::<_, ProgramRow>(
        r#"
        SELECT slug, name, short_name, icon, icon_url, website, server_base_url,
               reference_label, reference_format, reference_example,
               multi_ref_allowed, activation_threshold, supports_rove, capabilities,
               adif_my_sig, adif_my_sig_info, adif_sig_field, adif_sig_info_field,
               data_entry_label, data_entry_placeholder, data_entry_format,
               sort_order, is_active, created_at, updated_at
        FROM programs
        WHERE slug = $1 AND is_active = true
        "#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Get the version (max updated_at as epoch seconds) for active programs.
pub async fn get_programs_version(pool: &PgPool) -> Result<i64, AppError> {
    let version: Option<i64> = sqlx::query_scalar(
        "SELECT EXTRACT(EPOCH FROM MAX(updated_at))::bigint FROM programs WHERE is_active = true",
    )
    .fetch_one(pool)
    .await?;

    Ok(version.unwrap_or(0))
}
