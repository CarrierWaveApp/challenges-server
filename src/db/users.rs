use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::User;

pub async fn get_user_by_callsign(pool: &PgPool, callsign: &str) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT id, callsign, created_at
        FROM users
        WHERE callsign = $1
        "#,
    )
    .bind(callsign)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

pub async fn get_user_by_id(pool: &PgPool, user_id: Uuid) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT id, callsign, created_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

pub async fn search_users(pool: &PgPool, query: &str, limit: i64) -> Result<Vec<User>, AppError> {
    let pattern = format!("%{}%", query.to_uppercase());
    let users = sqlx::query_as::<_, User>(
        r#"
        SELECT id, callsign, created_at
        FROM users
        WHERE UPPER(callsign) LIKE $1
        ORDER BY callsign
        LIMIT $2
        "#,
    )
    .bind(&pattern)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(users)
}

/// Delete a user account and all associated data.
/// Deletes from callsign-based tables (no FK cascade) first,
/// then deletes the user row (which cascades to friend_requests,
/// friendships, friend_invites, and activities).
/// Also deletes participant records (auth tokens).
pub async fn delete_user_account(pool: &PgPool, callsign: &str) -> Result<u64, AppError> {
    let callsign_upper = callsign.to_uppercase();

    let mut tx = pool.begin().await?;

    // Delete from callsign-based tables (no FK cascade from users)
    sqlx::query("DELETE FROM participants WHERE callsign = $1")
        .bind(&callsign_upper)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM challenge_participants WHERE callsign = $1")
        .bind(&callsign_upper)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM progress WHERE callsign = $1")
        .bind(&callsign_upper)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM earned_badges WHERE callsign = $1")
        .bind(&callsign_upper)
        .execute(&mut *tx)
        .await?;

    // Delete user row (cascades to: friend_requests, friendships,
    // friend_invites, activities)
    let result = sqlx::query("DELETE FROM users WHERE callsign = $1")
        .bind(&callsign_upper)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(result.rows_affected())
}

pub async fn get_user_counts(pool: &PgPool) -> Result<(i64, i64, i64), AppError> {
    let row = sqlx::query_as::<_, (i64, i64, i64)>(
        r#"
        SELECT
            COUNT(*) AS total,
            COUNT(*) FILTER (WHERE created_at >= NOW() - INTERVAL '7 days') AS last_7,
            COUNT(*) FILTER (WHERE created_at >= NOW() - INTERVAL '30 days') AS last_30
        FROM users
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn get_active_users_by_hour(
    pool: &PgPool,
    days: i32,
) -> Result<Vec<crate::models::UserCountByHour>, AppError> {
    let rows = sqlx::query_as::<_, crate::models::UserCountByHour>(
        r#"
        SELECT date_trunc('hour', created_at) AS hour,
               COUNT(DISTINCT callsign) AS count
        FROM activities
        WHERE created_at >= NOW() - make_interval(days => $1)
        GROUP BY hour
        ORDER BY hour
        "#,
    )
    .bind(days)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Change a user's callsign across all callsign-keyed tables in a single transaction.
/// Records the change in callsign_history for audit.
/// Does NOT update pota_activations or pota_hunter_qsos (external data).
pub async fn change_callsign(
    pool: &PgPool,
    user_id: Uuid,
    old_callsign: &str,
    new_callsign: &str,
) -> Result<User, AppError> {
    let old_upper = old_callsign.to_uppercase();
    let new_upper = new_callsign.to_uppercase();

    // Check if new callsign is already taken
    let existing = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM users WHERE callsign = $1 AND id != $2",
    )
    .bind(&new_upper)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if existing.is_some() {
        return Err(AppError::CallsignTaken {
            callsign: new_upper,
        });
    }

    let mut tx = pool.begin().await?;

    // Update all callsign-keyed tables
    sqlx::query("UPDATE participants SET callsign = $1 WHERE callsign = $2")
        .bind(&new_upper)
        .bind(&old_upper)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE challenge_participants SET callsign = $1 WHERE callsign = $2")
        .bind(&new_upper)
        .bind(&old_upper)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE progress SET callsign = $1 WHERE callsign = $2")
        .bind(&new_upper)
        .bind(&old_upper)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE earned_badges SET callsign = $1 WHERE callsign = $2")
        .bind(&new_upper)
        .bind(&old_upper)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE activities SET callsign = $1 WHERE callsign = $2")
        .bind(&new_upper)
        .bind(&old_upper)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE club_members SET callsign = $1 WHERE callsign = $2")
        .bind(&new_upper)
        .bind(&old_upper)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE events SET submitted_by = $1 WHERE submitted_by = $2")
        .bind(&new_upper)
        .bind(&old_upper)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE spots SET callsign = $1 WHERE callsign = $2")
        .bind(&new_upper)
        .bind(&old_upper)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE upload_error_telemetry SET callsign = $1 WHERE callsign = $2")
        .bind(&new_upper)
        .bind(&old_upper)
        .execute(&mut *tx)
        .await?;

    // Update the users table last
    let user = sqlx::query_as::<_, User>(
        r#"
        UPDATE users SET callsign = $1
        WHERE id = $2
        RETURNING id, callsign, created_at
        "#,
    )
    .bind(&new_upper)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    // Record the change in history
    sqlx::query(
        r#"
        INSERT INTO callsign_history (user_id, old_callsign, new_callsign)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(user_id)
    .bind(&old_upper)
    .bind(&new_upper)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(user)
}

pub async fn get_or_create_user(pool: &PgPool, callsign: &str) -> Result<User, AppError> {
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (callsign)
        VALUES ($1)
        ON CONFLICT (callsign) DO UPDATE SET callsign = EXCLUDED.callsign
        RETURNING id, callsign, created_at
        "#,
    )
    .bind(callsign)
    .fetch_one(pool)
    .await?;

    Ok(user)
}
