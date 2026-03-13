use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::User;

#[allow(dead_code)]
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
