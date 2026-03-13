use chrono::{DateTime, Utc};
use rand::Rng;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::InviteToken;

fn generate_invite_token() -> String {
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = (0..24)
        .map(|_| {
            let idx = rng.gen_range(0..36);
            if idx < 10 {
                (b'0' + idx) as char
            } else {
                (b'a' + idx - 10) as char
            }
        })
        .collect();
    format!("inv_{}", chars.into_iter().collect::<String>())
}

pub async fn create_invite(
    pool: &PgPool,
    challenge_id: Uuid,
    max_uses: Option<i32>,
    expires_at: Option<DateTime<Utc>>,
) -> Result<InviteToken, AppError> {
    let token = generate_invite_token();

    let invite = sqlx::query_as::<_, InviteToken>(
        r#"
        INSERT INTO invite_tokens (token, challenge_id, max_uses, expires_at)
        VALUES ($1, $2, $3, $4)
        RETURNING token, challenge_id, max_uses, use_count, expires_at, created_at
        "#,
    )
    .bind(&token)
    .bind(challenge_id)
    .bind(max_uses)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;

    Ok(invite)
}

pub async fn list_invites(pool: &PgPool, challenge_id: Uuid) -> Result<Vec<InviteToken>, AppError> {
    let invites = sqlx::query_as::<_, InviteToken>(
        r#"
        SELECT token, challenge_id, max_uses, use_count, expires_at, created_at
        FROM invite_tokens
        WHERE challenge_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(challenge_id)
    .fetch_all(pool)
    .await?;

    Ok(invites)
}

#[allow(dead_code)]
pub async fn get_invite(pool: &PgPool, token: &str) -> Result<Option<InviteToken>, AppError> {
    let invite = sqlx::query_as::<_, InviteToken>(
        r#"
        SELECT token, challenge_id, max_uses, use_count, expires_at, created_at
        FROM invite_tokens
        WHERE token = $1
        "#,
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;

    Ok(invite)
}

pub async fn delete_invite(pool: &PgPool, token: &str) -> Result<bool, AppError> {
    let result = sqlx::query("DELETE FROM invite_tokens WHERE token = $1")
        .bind(token)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}
