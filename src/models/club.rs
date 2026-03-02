use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Database row types
// ---------------------------------------------------------------------------

/// Database row for the clubs table.
#[derive(Debug, Clone, FromRow)]
pub struct Club {
    pub id: Uuid,
    pub name: String,
    pub callsign: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Database row for the club_members table.
#[derive(Debug, Clone, FromRow)]
pub struct ClubMember {
    pub id: Uuid,
    pub club_id: Uuid,
    pub callsign: String,
    pub role: String,
    pub joined_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// API response types
// ---------------------------------------------------------------------------

/// Summary response for club list views (includes member count).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClubResponse {
    pub id: Uuid,
    pub name: String,
    pub callsign: Option<String>,
    pub description: Option<String>,
    pub member_count: i64,
}

/// Detailed response for a single club (includes full member list).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClubDetailResponse {
    pub id: Uuid,
    pub name: String,
    pub callsign: Option<String>,
    pub description: Option<String>,
    pub members: Vec<ClubMemberResponse>,
}

/// Response for a single club member with enriched presence data.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClubMemberResponse {
    pub callsign: String,
    pub role: String,
    pub joined_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_grid: Option<String>,
    pub is_carrier_wave_user: bool,
}

/// Real-time status for a club member.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberStatusResponse {
    pub callsign: String,
    pub status: MemberOnlineStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spot_info: Option<SpotInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<DateTime<Utc>>,
}

/// Online status classification for a club member.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MemberOnlineStatus {
    OnAir,
    RecentlyActive,
    Inactive,
}

/// Spot information attached to a member status.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotInfo {
    pub frequency: f64,
    pub mode: Option<String>,
    pub source: String,
    pub spotted_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// API request types
// ---------------------------------------------------------------------------

/// Request body for POST /v1/clubs.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateClubRequest {
    pub name: String,
    pub callsign: Option<String>,
    pub description: Option<String>,
}

/// Request body for PATCH /v1/clubs/:id.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateClubRequest {
    pub name: Option<String>,
    pub callsign: Option<String>,
    pub description: Option<String>,
}

/// Request body for POST /v1/clubs/:id/members.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddMembersRequest {
    pub members: Vec<AddMemberEntry>,
}

/// A single member entry in an add-members request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddMemberEntry {
    pub callsign: String,
    #[serde(default = "default_member_role")]
    pub role: String,
}

fn default_member_role() -> String {
    "member".to_string()
}

/// Request body for PATCH /v1/clubs/:id/members/:callsign.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMemberRoleRequest {
    pub role: String,
}
