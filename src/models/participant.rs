use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct Participant {
    pub id: Uuid,
    pub callsign: String,
    pub device_token: String,
    pub device_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ChallengeParticipant {
    pub id: Uuid,
    pub challenge_id: Uuid,
    pub callsign: String,
    pub invite_token: Option<String>,
    pub joined_at: DateTime<Utc>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinChallengeRequest {
    pub callsign: String,
    pub device_name: Option<String>,
    pub invite_token: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinChallengeResponse {
    pub participation_id: Uuid,
    pub device_token: String,
    pub joined_at: DateTime<Utc>,
    pub status: String,
    pub historical_allowed: bool,
}
