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
use crate::models::{
    AdminStatsResponse, RegisterRequest, RegisterResponse, UpdateCallsignRequest,
    UpdateCallsignResponse, UserCountByHour,
};
use axum::http::StatusCode;
use axum::Extension;

/// GET /v1/admin/stats — aggregate user statistics (admin only)
pub async fn admin_stats(
    State(pool): State<PgPool>,
) -> Result<Json<DataResponse<AdminStatsResponse>>, AppError> {
    let (total, last_7, last_30) = db::get_user_counts(&pool).await?;

    Ok(Json(DataResponse {
        data: AdminStatsResponse {
            total_users: total,
            users_last_7_days: last_7,
            users_last_30_days: last_30,
        },
    }))
}

#[derive(Debug, Deserialize)]
pub struct UserCountsByHourQuery {
    pub days: Option<i32>,
}

/// GET /v1/admin/stats/users-by-hour — active users per hour (admin only)
pub async fn admin_users_by_hour(
    State(pool): State<PgPool>,
    Query(query): Query<UserCountsByHourQuery>,
) -> Result<Json<DataResponse<Vec<UserCountByHour>>>, AppError> {
    let days = query.days.unwrap_or(30).clamp(1, 365);
    let data = db::get_active_users_by_hour(&pool, days).await?;

    Ok(Json(DataResponse { data }))
}

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

/// PUT /v1/account/callsign
/// Update the authenticated user's callsign across all tables.
/// Returns the new callsign and list of previous callsigns.
pub async fn update_callsign(
    State(pool): State<PgPool>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<UpdateCallsignRequest>,
) -> Result<Json<DataResponse<UpdateCallsignResponse>>, AppError> {
    let new_callsign = body.new_callsign.trim().to_uppercase();

    if new_callsign.is_empty() {
        return Err(AppError::Validation {
            message: "newCallsign is required".to_string(),
        });
    }

    // Look up the user by current callsign to get user_id
    let user = db::get_user_by_callsign(&pool, &auth.callsign)
        .await?
        .ok_or(AppError::UserNotFound {
            user_id: auth.participant_id,
        })?;

    let (updated_user, previous_callsigns) =
        db::update_callsign(&pool, user.id, &auth.callsign, &new_callsign).await?;

    Ok(Json(DataResponse {
        data: UpdateCallsignResponse {
            user_id: updated_user.id,
            callsign: updated_user.callsign,
            previous_callsigns,
        },
    }))
}
