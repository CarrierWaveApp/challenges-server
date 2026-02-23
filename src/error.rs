// src/error.rs
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Challenge not found")]
    ChallengeNotFound { challenge_id: Uuid },

    #[error("Badge not found")]
    BadgeNotFound { badge_id: Uuid },

    #[error("Invite not found")]
    InviteNotFound { token: String },

    #[error("User not found")]
    UserNotFound { user_id: Uuid },

    #[error("Friend invite not found or expired")]
    FriendInviteNotFound { token: String },

    #[error("Friend invite has already been used")]
    FriendInviteUsed { token: String },

    #[error("Program not found")]
    ProgramNotFound { slug: String },

    #[error("Friend request not found")]
    FriendRequestNotFound { request_id: Uuid },

    #[error("Friendship not found")]
    FriendshipNotFound { friendship_id: Uuid },

    #[error("Already friends with this user")]
    AlreadyFriends,

    #[error("Friend request already exists")]
    FriendRequestExists,

    #[error("Cannot send friend request to yourself")]
    CannotFriendSelf,

    #[error("Already joined this challenge")]
    AlreadyJoined,

    #[error("Not participating in this challenge")]
    NotParticipating,

    #[error("Invite token required")]
    InviteRequired,

    #[error("Invite token expired")]
    InviteExpired,

    #[error("Invite token exhausted")]
    InviteExhausted,

    #[error("Challenge at maximum participants")]
    MaxParticipants,

    #[error("Challenge has ended")]
    ChallengeEnded,

    #[error("Invalid or revoked token")]
    InvalidToken,

    #[error("Forbidden")]
    Forbidden,

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Validation error: {message}")]
    Validation { message: String },

    #[error("Database error")]
    Database(#[from] sqlx::Error),

    #[error("Internal server error")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, details) = match &self {
            Self::ChallengeNotFound { challenge_id } => (
                StatusCode::NOT_FOUND,
                "CHALLENGE_NOT_FOUND",
                Some(serde_json::json!({ "challengeId": challenge_id })),
            ),
            Self::BadgeNotFound { badge_id } => (
                StatusCode::NOT_FOUND,
                "BADGE_NOT_FOUND",
                Some(serde_json::json!({ "badgeId": badge_id })),
            ),
            Self::ProgramNotFound { slug } => (
                StatusCode::NOT_FOUND,
                "PROGRAM_NOT_FOUND",
                Some(serde_json::json!({ "slug": slug })),
            ),
            Self::InviteNotFound { token } => (
                StatusCode::NOT_FOUND,
                "INVITE_NOT_FOUND",
                Some(serde_json::json!({ "token": token })),
            ),
            Self::UserNotFound { user_id } => (
                StatusCode::NOT_FOUND,
                "USER_NOT_FOUND",
                Some(serde_json::json!({ "userId": user_id })),
            ),
            Self::FriendInviteNotFound { token } => (
                StatusCode::NOT_FOUND,
                "FRIEND_INVITE_NOT_FOUND",
                Some(serde_json::json!({ "token": token })),
            ),
            Self::FriendInviteUsed { token } => (
                StatusCode::GONE,
                "FRIEND_INVITE_USED",
                Some(serde_json::json!({ "token": token })),
            ),
            Self::FriendRequestNotFound { request_id } => (
                StatusCode::NOT_FOUND,
                "FRIEND_REQUEST_NOT_FOUND",
                Some(serde_json::json!({ "requestId": request_id })),
            ),
            Self::FriendshipNotFound { friendship_id } => (
                StatusCode::NOT_FOUND,
                "FRIENDSHIP_NOT_FOUND",
                Some(serde_json::json!({ "friendshipId": friendship_id })),
            ),
            Self::AlreadyFriends => (StatusCode::CONFLICT, "ALREADY_FRIENDS", None),
            Self::FriendRequestExists => (StatusCode::CONFLICT, "FRIEND_REQUEST_EXISTS", None),
            Self::CannotFriendSelf => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "CANNOT_FRIEND_SELF",
                None,
            ),
            Self::AlreadyJoined => (StatusCode::CONFLICT, "ALREADY_JOINED", None),
            Self::NotParticipating => (StatusCode::FORBIDDEN, "NOT_PARTICIPATING", None),
            Self::InviteRequired => (StatusCode::FORBIDDEN, "INVITE_REQUIRED", None),
            Self::InviteExpired => (StatusCode::FORBIDDEN, "INVITE_EXPIRED", None),
            Self::InviteExhausted => (StatusCode::FORBIDDEN, "INVITE_EXHAUSTED", None),
            Self::MaxParticipants => (StatusCode::FORBIDDEN, "MAX_PARTICIPANTS", None),
            Self::ChallengeEnded => (StatusCode::BAD_REQUEST, "CHALLENGE_ENDED", None),
            Self::InvalidToken => (StatusCode::UNAUTHORIZED, "INVALID_TOKEN", None),
            Self::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN", None),
            Self::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED", None),
            Self::Validation { .. } => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", None),
            Self::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", None),
            Self::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", None),
        };

        let body = ErrorResponse {
            error: ErrorBody {
                code,
                message: self.to_string(),
                details,
            },
        };

        (status, Json(body)).into_response()
    }
}
