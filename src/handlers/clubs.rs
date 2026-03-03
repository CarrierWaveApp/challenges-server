use axum::extract::{Extension, Query, State};
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::extractors::{Json, Path};
use sqlx::PgPool;

use crate::auth::AuthContext;
use crate::db;
use crate::error::AppError;
use crate::models::club::{
    ClubDetailResponse, ClubMemberResponse, ClubResponse, MemberOnlineStatus,
    MemberStatusResponse, SpotInfo, UpdateClubNotesRequest,
};

use super::DataResponse;

/// GET /v1/clubs
/// Get clubs for the authenticated user.
pub async fn get_clubs(
    State(pool): State<PgPool>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DataResponse<Vec<ClubResponse>>>, AppError> {
    let clubs = db::clubs::get_clubs_for_callsign(&pool, &auth.callsign).await?;

    let data = clubs
        .into_iter()
        .map(|c| ClubResponse {
            id: c.id,
            name: c.name,
            callsign: c.callsign,
            description: c.description,
            notes_url: c.notes_url,
            notes_title: c.notes_title,
            member_count: c.member_count,
        })
        .collect();

    Ok(Json(DataResponse { data }))
}

/// GET /v1/clubs/:id
/// Get club details (requires membership).
pub async fn get_club_details(
    State(pool): State<PgPool>,
    Extension(auth): Extension<AuthContext>,
    Path(club_id): Path<Uuid>,
) -> Result<Json<DataResponse<ClubDetailResponse>>, AppError> {
    // Verify caller is a member
    if !db::clubs::is_club_member(&pool, club_id, &auth.callsign).await? {
        return Err(AppError::Forbidden);
    }

    let club = db::clubs::get_club_detail(&pool, club_id)
        .await?
        .ok_or(AppError::ClubNotFound { club_id })?;

    let members = db::clubs::get_club_members_enriched(&pool, club_id).await?;

    let member_responses = members
        .into_iter()
        .map(|m| ClubMemberResponse {
            callsign: m.callsign,
            role: m.role,
            joined_at: m.joined_at,
            last_seen_at: m.last_seen_at,
            last_grid: m.last_grid,
            is_carrier_wave_user: m.is_carrier_wave_user,
        })
        .collect();

    Ok(Json(DataResponse {
        data: ClubDetailResponse {
            id: club.id,
            name: club.name,
            callsign: club.callsign,
            description: club.description,
            notes_url: club.notes_url,
            notes_title: club.notes_title,
            members: member_responses,
        },
    }))
}

#[derive(serde::Deserialize)]
pub struct ClubActivityQuery {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClubActivityResponse {
    pub items: Vec<ClubActivityItem>,
    pub pagination: ClubActivityPagination,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClubActivityItem {
    pub id: Uuid,
    pub callsign: String,
    pub activity_type: String,
    pub timestamp: chrono::DateTime<Utc>,
    pub details: serde_json::Value,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClubActivityPagination {
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

/// GET /v1/clubs/:id/activity
/// Get activity feed for a club (requires membership).
pub async fn get_club_activity(
    State(pool): State<PgPool>,
    Extension(auth): Extension<AuthContext>,
    Path(club_id): Path<Uuid>,
    Query(params): Query<ClubActivityQuery>,
) -> Result<Json<DataResponse<ClubActivityResponse>>, AppError> {
    // Verify membership
    if !db::clubs::is_club_member(&pool, club_id, &auth.callsign).await? {
        return Err(AppError::Forbidden);
    }

    let limit = params.limit.unwrap_or(20).min(100).max(1);

    // Parse cursor (ISO 8601 timestamp)
    let cursor = params.cursor.as_deref().and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    });

    // Fetch one extra to determine has_more
    let rows = db::clubs::get_club_activity(&pool, club_id, cursor, limit + 1).await?;

    let has_more = rows.len() as i64 > limit;
    let truncated: Vec<_> = rows.into_iter().take(limit as usize).collect();

    let next_cursor = if has_more {
        truncated.last().map(|row| row.created_at.to_rfc3339())
    } else {
        None
    };

    let items = truncated
        .into_iter()
        .map(|row| ClubActivityItem {
            id: row.id,
            callsign: row.callsign,
            activity_type: row.activity_type,
            timestamp: row.timestamp,
            details: row.details,
            created_at: row.created_at,
        })
        .collect();

    Ok(Json(DataResponse {
        data: ClubActivityResponse {
            items,
            pagination: ClubActivityPagination {
                has_more,
                next_cursor,
            },
        },
    }))
}

/// GET /v1/clubs/:id/status
/// Get real-time status for club members (requires membership).
pub async fn get_club_status(
    State(pool): State<PgPool>,
    Extension(auth): Extension<AuthContext>,
    Path(club_id): Path<Uuid>,
) -> Result<Json<DataResponse<Vec<MemberStatusResponse>>>, AppError> {
    // Verify membership
    if !db::clubs::is_club_member(&pool, club_id, &auth.callsign).await? {
        return Err(AppError::Forbidden);
    }

    let members = db::clubs::get_club_members_enriched(&pool, club_id).await?;
    let now = Utc::now();
    let spot_cutoff = now - Duration::minutes(30);
    let active_cutoff = now - Duration::minutes(15);

    let mut statuses = Vec::with_capacity(members.len());

    for member in &members {
        // Check for recent spot (on_air)
        let spot = find_recent_spot(&pool, &member.callsign, spot_cutoff).await?;

        let (status, spot_info) = if let Some(spot) = spot {
            (
                MemberOnlineStatus::OnAir,
                Some(SpotInfo {
                    frequency: spot.frequency_khz,
                    mode: Some(spot.mode),
                    source: format!("{:?}", spot.source).to_lowercase(),
                    spotted_at: spot.spotted_at,
                }),
            )
        } else if member
            .last_seen_at
            .is_some_and(|seen| seen > active_cutoff)
        {
            (MemberOnlineStatus::RecentlyActive, None)
        } else {
            (MemberOnlineStatus::Inactive, None)
        };

        statuses.push(MemberStatusResponse {
            callsign: member.callsign.clone(),
            status,
            spot_info,
            last_seen_at: member.last_seen_at,
        });
    }

    Ok(Json(DataResponse { data: statuses }))
}

/// PUT /v1/clubs/:id/notes
/// Update club notes (requires club admin role).
pub async fn update_club_notes(
    State(pool): State<PgPool>,
    Extension(auth): Extension<AuthContext>,
    Path(club_id): Path<Uuid>,
    Json(body): Json<UpdateClubNotesRequest>,
) -> Result<Json<DataResponse<ClubResponse>>, AppError> {
    // Verify caller is a club member
    if !db::clubs::is_club_member(&pool, club_id, &auth.callsign).await? {
        return Err(AppError::Forbidden);
    }

    // Verify caller is a club admin
    let members = db::clubs::get_club_members_enriched(&pool, club_id).await?;
    let caller_upper = auth.callsign.to_uppercase();
    let caller = members.iter().find(|m| m.callsign == caller_upper);

    match caller {
        Some(m) if m.role == "admin" => {}
        _ => return Err(AppError::Forbidden),
    }

    // Validate URL starts with https:// if provided
    if let Some(ref url) = body.notes_url {
        if !url.starts_with("https://") {
            return Err(AppError::Validation {
                message: "notes_url must start with https://".to_string(),
            });
        }
    }

    // Update notes
    let club = db::clubs::update_club_notes(
        &pool,
        club_id,
        body.notes_url.as_deref(),
        body.notes_title.as_deref(),
    )
    .await?
    .ok_or(AppError::ClubNotFound { club_id })?;

    // Get member count for response
    let member_count = members.len() as i64;

    Ok(Json(DataResponse {
        data: ClubResponse {
            id: club.id,
            name: club.name,
            callsign: club.callsign,
            description: club.description,
            notes_url: club.notes_url,
            notes_title: club.notes_title,
            member_count,
        },
    }))
}

/// Internal: partial spot row for status queries.
#[derive(sqlx::FromRow)]
struct SpotSummary {
    frequency_khz: f64,
    mode: String,
    source: crate::models::spot::SpotSource,
    spotted_at: chrono::DateTime<Utc>,
}

/// Find the most recent unexpired spot for a callsign since `since`.
async fn find_recent_spot(
    pool: &PgPool,
    callsign: &str,
    since: chrono::DateTime<Utc>,
) -> Result<Option<SpotSummary>, AppError> {
    let row = sqlx::query_as::<_, SpotSummary>(
        r#"
        SELECT frequency_khz, mode, source, spotted_at
        FROM spots
        WHERE callsign = $1
          AND spotted_at >= $2
          AND expires_at > now()
        ORDER BY spotted_at DESC
        LIMIT 1
        "#,
    )
    .bind(callsign)
    .bind(since)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}
