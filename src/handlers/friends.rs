use axum::{
    extract::{Extension, State},
    http::StatusCode,
};

use crate::extractors::Json;
use sqlx::PgPool;

use crate::auth::AuthContext;
use crate::config::Config;
use crate::db;
use crate::error::AppError;
use crate::models::{CreateFriendRequestBody, FriendInviteResponse, FriendRequestResponse};

use super::DataResponse;

/// GET /v1/friends/invite-link
/// Generate a new friend invite link for the authenticated user
pub async fn get_invite_link(
    State(pool): State<PgPool>,
    Extension(config): Extension<Config>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DataResponse<FriendInviteResponse>>, AppError> {
    // Get or create user record for this callsign
    let user = db::get_or_create_user(&pool, &auth.callsign).await?;

    // Create a new invite
    let invite = db::create_friend_invite(&pool, user.id, config.invite_expiry_days).await?;

    Ok(Json(DataResponse {
        data: invite.into_response(&config.invite_base_url),
    }))
}

/// POST /v1/friends/requests
/// Create a friend request either by user ID or invite token
pub async fn create_friend_request(
    State(pool): State<PgPool>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<CreateFriendRequestBody>,
) -> Result<(StatusCode, Json<DataResponse<FriendRequestResponse>>), AppError> {
    // Get or create user record for the sender
    let sender = db::get_or_create_user(&pool, &auth.callsign).await?;

    // Determine the target user
    let target_user_id = match (&body.to_user_id, &body.invite_token) {
        (Some(user_id), None) => {
            // Direct user ID specified
            let target = db::get_user_by_id(&pool, *user_id)
                .await?
                .ok_or(AppError::UserNotFound { user_id: *user_id })?;
            target.id
        }
        (None, Some(token)) => {
            // Invite token specified
            let invite = db::get_valid_friend_invite(&pool, token)
                .await?
                .ok_or_else(|| AppError::FriendInviteNotFound {
                    token: token.clone(),
                })?;

            // Mark the invite as used
            db::mark_invite_used(&pool, token, sender.id).await?;

            invite.user_id
        }
        (Some(_), Some(_)) => {
            return Err(AppError::Validation {
                message: "Provide either toUserId or inviteToken, not both".to_string(),
            });
        }
        (None, None) => {
            return Err(AppError::Validation {
                message: "Either toUserId or inviteToken is required".to_string(),
            });
        }
    };

    // Cannot friend yourself
    if sender.id == target_user_id {
        return Err(AppError::CannotFriendSelf);
    }

    // Check if already friends
    if db::are_friends(&pool, sender.id, target_user_id).await? {
        return Err(AppError::AlreadyFriends);
    }

    // Check if a pending request already exists (in either direction)
    if db::get_pending_request_between(&pool, sender.id, target_user_id)
        .await?
        .is_some()
    {
        return Err(AppError::FriendRequestExists);
    }

    // Create the friend request
    let request = db::create_friend_request(&pool, sender.id, target_user_id).await?;

    Ok((
        StatusCode::CREATED,
        Json(DataResponse {
            data: request.into(),
        }),
    ))
}
