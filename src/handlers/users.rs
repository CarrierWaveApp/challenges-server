use axum::extract::{Query, State};
use serde::Deserialize;
use sqlx::PgPool;

use crate::db;
use crate::error::AppError;
use crate::extractors::Json;
use crate::models::UserSearchResponse;

use super::DataResponse;

#[derive(Debug, Deserialize)]
pub struct SearchUsersQuery {
    pub q: String,
}

/// GET /v1/users/search?q=...
/// Search for users by callsign (public, no auth required)
pub async fn search_users(
    State(pool): State<PgPool>,
    Query(query): Query<SearchUsersQuery>,
) -> Result<Json<DataResponse<Vec<UserSearchResponse>>>, AppError> {
    if query.q.len() < 2 {
        return Ok(Json(DataResponse { data: vec![] }));
    }

    let users = db::search_users(&pool, &query.q, 20).await?;

    let results: Vec<UserSearchResponse> = users.into_iter().map(|u| u.into()).collect();

    Ok(Json(DataResponse { data: results }))
}

use crate::auth::AuthContext;
use crate::models::{RegisterRequest, RegisterResponse};
use axum::http::StatusCode;
use axum::Extension;

/// POST /v1/register
/// Register a user so they appear in friend search and get an auth token.
/// Creates rows in both users and participants tables.
pub async fn register(
    State(pool): State<PgPool>,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<DataResponse<RegisterResponse>>), AppError> {
    if body.callsign.trim().is_empty() {
        return Err(AppError::Validation {
            message: "callsign is required".to_string(),
        });
    }

    // Create user record (for friend search)
    let user = db::get_or_create_user(&pool, &body.callsign).await?;

    // Create participant record (for auth token)
    let (participant, _is_new) =
        db::get_or_create_participant(&pool, &body.callsign, body.device_name.as_deref()).await?;

    Ok((
        StatusCode::CREATED,
        Json(DataResponse {
            data: RegisterResponse {
                user_id: user.id,
                device_token: participant.device_token,
            },
        }),
    ))
}

/// DELETE /v1/account
/// Delete the authenticated user's account and all associated data.
pub async fn delete_account(
    State(pool): State<PgPool>,
    Extension(auth): Extension<AuthContext>,
) -> Result<StatusCode, AppError> {
    let rows = db::delete_user_account(&pool, &auth.callsign).await?;

    if rows == 0 {
        return Err(AppError::UserNotFound {
            user_id: auth.participant_id,
        });
    }

    Ok(StatusCode::NO_CONTENT)
}
