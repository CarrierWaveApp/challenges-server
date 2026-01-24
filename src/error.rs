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
            Self::AlreadyJoined => (StatusCode::CONFLICT, "ALREADY_JOINED", None),
            Self::NotParticipating => (StatusCode::FORBIDDEN, "NOT_PARTICIPATING", None),
            Self::InviteRequired => (StatusCode::FORBIDDEN, "INVITE_REQUIRED", None),
            Self::InviteExpired => (StatusCode::FORBIDDEN, "INVITE_EXPIRED", None),
            Self::InviteExhausted => (StatusCode::FORBIDDEN, "INVITE_EXHAUSTED", None),
            Self::MaxParticipants => (StatusCode::FORBIDDEN, "MAX_PARTICIPANTS", None),
            Self::ChallengeEnded => (StatusCode::BAD_REQUEST, "CHALLENGE_ENDED", None),
            Self::InvalidToken => (StatusCode::UNAUTHORIZED, "INVALID_TOKEN", None),
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
