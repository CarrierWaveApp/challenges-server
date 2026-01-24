use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use sqlx::PgPool;

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub callsign: String,
    pub participant_id: uuid::Uuid,
}

pub async fn optional_auth(
    State(pool): State<PgPool>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    if let Some(auth_header) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                if let Some(ctx) = validate_token(&pool, token).await? {
                    req.extensions_mut().insert(ctx);
                }
            }
        }
    }
    Ok(next.run(req).await)
}

pub async fn require_auth(
    State(pool): State<PgPool>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = req
        .headers()
        .get("authorization")
        .ok_or(AppError::InvalidToken)?;

    let auth_str = auth_header.to_str().map_err(|_| AppError::InvalidToken)?;

    let token = auth_str
        .strip_prefix("Bearer ")
        .ok_or(AppError::InvalidToken)?;

    let ctx = validate_token(&pool, token)
        .await?
        .ok_or(AppError::InvalidToken)?;

    req.extensions_mut().insert(ctx);
    Ok(next.run(req).await)
}

async fn validate_token(pool: &PgPool, token: &str) -> Result<Option<AuthContext>, AppError> {
    let participant = sqlx::query!(
        r#"
        UPDATE participants
        SET last_seen_at = now()
        WHERE device_token = $1
        RETURNING id, callsign
        "#,
        token
    )
    .fetch_optional(pool)
    .await?;

    Ok(participant.map(|p| AuthContext {
        callsign: p.callsign,
        participant_id: p.id,
    }))
}

pub async fn require_admin(
    State(admin_token): State<String>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = req
        .headers()
        .get("authorization")
        .ok_or(AppError::InvalidToken)?;

    let auth_str = auth_header.to_str().map_err(|_| AppError::InvalidToken)?;

    let token = auth_str
        .strip_prefix("Bearer ")
        .ok_or(AppError::InvalidToken)?;

    if token != admin_token {
        return Err(AppError::InvalidToken);
    }

    Ok(next.run(req).await)
}
