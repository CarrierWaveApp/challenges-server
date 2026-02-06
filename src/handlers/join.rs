use axum::{
    extract::{Extension, State},
    http::StatusCode,
};

use crate::extractors::{Json, Path};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::db;
use crate::error::AppError;
use crate::models::{JoinChallengeRequest, JoinChallengeResponse};

use super::DataResponse;

pub async fn join_challenge(
    State(pool): State<PgPool>,
    Path(challenge_id): Path<Uuid>,
    _auth: Option<Extension<AuthContext>>,
    Json(req): Json<JoinChallengeRequest>,
) -> Result<(StatusCode, Json<DataResponse<JoinChallengeResponse>>), AppError> {
    let challenge = db::get_challenge(&pool, challenge_id)
        .await?
        .ok_or(AppError::ChallengeNotFound { challenge_id })?;

    if !challenge.is_active {
        return Err(AppError::ChallengeEnded);
    }

    if let Some(invite_config) = &challenge.invite_config {
        let requires_token = invite_config
            .get("requiresToken")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if requires_token && req.invite_token.is_none() {
            return Err(AppError::InviteRequired);
        }
    }

    let (mut participant, is_new) =
        db::get_or_create_participant(&pool, &req.callsign, req.device_name.as_deref()).await?;

    // Refresh token on re-join (allows token recovery)
    if !is_new {
        participant = db::refresh_participant_token(&pool, &req.callsign).await?;
    }

    let participation = db::join_challenge(
        &pool,
        challenge_id,
        &req.callsign,
        req.invite_token.as_deref(),
    )
    .await?;

    let historical_allowed = challenge
        .configuration
        .get("historicalQsosAllowed")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    Ok((
        StatusCode::CREATED,
        Json(DataResponse {
            data: JoinChallengeResponse {
                participation_id: participation.id,
                device_token: participant.device_token,
                joined_at: participation.joined_at,
                status: participation.status,
                historical_allowed,
            },
        }),
    ))
}

pub async fn leave_challenge(
    State(pool): State<PgPool>,
    Path(challenge_id): Path<Uuid>,
    Extension(auth): Extension<AuthContext>,
) -> Result<StatusCode, AppError> {
    let callsign_upper = auth.callsign.to_uppercase();

    sqlx::query("DELETE FROM progress WHERE challenge_id = $1 AND callsign = $2")
        .bind(challenge_id)
        .bind(&callsign_upper)
        .execute(&pool)
        .await?;

    let left = db::leave_challenge(&pool, challenge_id, &callsign_upper).await?;

    if left {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotParticipating)
    }
}
