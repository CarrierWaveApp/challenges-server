use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
};

use crate::extractors::Json;
use sqlx::PgPool;

use crate::auth::AuthContext;
use crate::db;
use crate::error::AppError;
use crate::models::activity::{ActivityResponse, FeedItemResponse, ReportActivityRequest};

use super::DataResponse;

/// POST /v1/activities
/// Report a notable activity.
pub async fn report_activity(
    State(pool): State<PgPool>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<ReportActivityRequest>,
) -> Result<(StatusCode, Json<DataResponse<ActivityResponse>>), AppError> {
    let user = db::get_or_create_user(&pool, &auth.callsign).await?;

    let activity = db::insert_activity(
        &pool,
        user.id,
        &auth.callsign,
        &body.activity_type,
        body.timestamp,
        &body.details,
    )
    .await?;

    let response: ActivityResponse = activity.into();
    Ok((StatusCode::CREATED, Json(DataResponse { data: response })))
}

/// DELETE /v1/activities/:id
/// Delete an activity (must be owned by the authenticated user).
pub async fn delete_activity(
    State(pool): State<PgPool>,
    Extension(auth): Extension<AuthContext>,
    Path(activity_id): Path<uuid::Uuid>,
) -> Result<StatusCode, AppError> {
    let user = db::get_or_create_user(&pool, &auth.callsign).await?;
    db::delete_activity(&pool, activity_id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub struct FeedQuery {
    pub limit: Option<i64>,
    pub filter: Option<String>,
    pub before: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedResponse {
    pub items: Vec<FeedItemResponse>,
    pub pagination: FeedPagination,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedPagination {
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

/// GET /v1/feed
/// Get activity feed from friends, with cursor-based pagination.
pub async fn get_feed(
    State(pool): State<PgPool>,
    Extension(auth): Extension<AuthContext>,
    Query(params): Query<FeedQuery>,
) -> Result<Json<DataResponse<FeedResponse>>, AppError> {
    let user = db::get_or_create_user(&pool, &auth.callsign).await?;

    let limit = params.limit.unwrap_or(50).min(100).max(1);

    // Parse cursor (ISO 8601 timestamp)
    let before = params.before.as_deref().and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    });

    // Fetch one extra to determine hasMore
    let rows = db::get_feed_for_user(&pool, user.id, limit + 1, before).await?;

    let has_more = rows.len() as i64 > limit;
    let truncated: Vec<_> = rows.into_iter().take(limit as usize).collect();

    let next_cursor = if has_more {
        truncated.last().map(|row| row.created_at.to_rfc3339())
    } else {
        None
    };

    let items: Vec<FeedItemResponse> = truncated.into_iter().map(Into::into).collect();

    Ok(Json(DataResponse {
        data: FeedResponse {
            items,
            pagination: FeedPagination {
                has_more,
                next_cursor,
            },
        },
    }))
}

/// GET /v1/clubs
/// Get clubs for user (stub: returns empty list)
pub async fn get_clubs(
    State(_pool): State<PgPool>,
    Extension(_auth): Extension<AuthContext>,
) -> Result<Json<DataResponse<Vec<serde_json::Value>>>, AppError> {
    Ok(Json(DataResponse { data: vec![] }))
}

/// GET /v1/clubs/:id
/// Get club details (stub: returns not found)
pub async fn get_club_details(
    State(_pool): State<PgPool>,
    Extension(_auth): Extension<AuthContext>,
    axum::extract::Path(_club_id): axum::extract::Path<uuid::Uuid>,
) -> Result<Json<DataResponse<serde_json::Value>>, AppError> {
    Err(AppError::Validation {
        message: "Clubs not yet implemented".to_string(),
    })
}
