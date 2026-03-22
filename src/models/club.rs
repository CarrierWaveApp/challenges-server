use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Database row types
// ---------------------------------------------------------------------------

/// Database row for the clubs table.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct Club {
    pub id: Uuid,
    pub name: String,
    pub callsign: Option<String>,
    pub description: Option<String>,
    pub notes_url: Option<String>,
    pub notes_title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Database row for the club_members table.
#[allow(dead_code)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes_title: Option<String>,
    pub member_count: i64,
}

/// Combined club + members response for the sync endpoint.
/// Returns all clubs with full member details in a single payload.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClubSyncResponse {
    pub id: Uuid,
    pub name: String,
    pub callsign: Option<String>,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes_title: Option<String>,
    pub member_count: i64,
    pub members: Vec<ClubMemberResponse>,
}

/// Detailed response for a single club (includes full member list).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClubDetailResponse {
    pub id: Uuid,
    pub name: String,
    pub callsign: Option<String>,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes_title: Option<String>,
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
    #[serde(default, deserialize_with = "deserialize_optional_nullable")]
    pub callsign: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_optional_nullable")]
    pub description: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_nullable",
        alias = "notes_url"
    )]
    pub notes_url: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_nullable",
        alias = "notes_title"
    )]
    pub notes_title: Option<Option<String>>,
}

/// Request body for PUT /v1/clubs/:id/notes.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateClubNotesRequest {
    pub notes_url: Option<String>,
    pub notes_title: Option<String>,
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

/// Deserialize a field as `Option<Option<T>>` where:
/// - field missing → `None` (don't update)
/// - field is `null` → `Some(None)` (set to NULL)
/// - field is a value → `Some(Some(value))` (set to value)
fn deserialize_optional_nullable<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// Request body for PATCH /v1/clubs/:id/members/:callsign.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMemberRoleRequest {
    pub role: String,
}

/// Response for POST /v1/admin/clubs/:id/import-notes.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportNotesResponse {
    pub imported: usize,
    pub skipped: usize,
    pub callsigns: Vec<String>,
}

// ---------------------------------------------------------------------------
// Membership monitor types
// ---------------------------------------------------------------------------

/// Database row for the club_membership_monitors table.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct MembershipMonitor {
    pub id: Uuid,
    pub club_id: Uuid,
    pub url: String,
    pub label: Option<String>,
    pub format: String,
    pub interval_hours: i32,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub last_status: Option<String>,
    pub last_member_count: Option<i32>,
    pub enabled: bool,
    pub remove_stale: bool,
    pub created_at: DateTime<Utc>,
}

/// API response for a membership monitor.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MembershipMonitorResponse {
    pub id: Uuid,
    pub club_id: Uuid,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub format: String,
    pub interval_hours: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_checked_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_member_count: Option<i32>,
    pub enabled: bool,
    pub remove_stale: bool,
    pub created_at: DateTime<Utc>,
}

impl From<MembershipMonitor> for MembershipMonitorResponse {
    fn from(m: MembershipMonitor) -> Self {
        Self {
            id: m.id,
            club_id: m.club_id,
            url: m.url,
            label: m.label,
            format: m.format,
            interval_hours: m.interval_hours,
            last_checked_at: m.last_checked_at,
            last_status: m.last_status,
            last_member_count: m.last_member_count,
            enabled: m.enabled,
            remove_stale: m.remove_stale,
            created_at: m.created_at,
        }
    }
}

/// Request body for POST /v1/admin/clubs/:id/monitors.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMonitorRequest {
    pub url: String,
    pub label: Option<String>,
    #[serde(default = "default_monitor_format")]
    pub format: String,
    #[serde(default = "default_interval_hours")]
    pub interval_hours: i32,
    #[serde(default)]
    pub remove_stale: bool,
}

fn default_monitor_format() -> String {
    "callsign_notes".to_string()
}

fn default_interval_hours() -> i32 {
    24
}

/// Request body for PUT /v1/admin/clubs/:id/monitors/:monitor_id.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMonitorRequest {
    pub url: Option<String>,
    pub label: Option<Option<String>>,
    pub format: Option<String>,
    pub interval_hours: Option<i32>,
    pub enabled: Option<bool>,
    pub remove_stale: Option<bool>,
}

/// Response for a monitor check trigger.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorCheckResponse {
    pub added: usize,
    pub removed: usize,
    pub total: usize,
    pub status: String,
}
