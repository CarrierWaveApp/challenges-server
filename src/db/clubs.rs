use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::club::{Club, ClubMember};

// ---------------------------------------------------------------------------
// Helper structs for enriched queries
// ---------------------------------------------------------------------------

/// Club row with member count for list views.
#[derive(Debug, Clone, FromRow)]
pub struct ClubWithCount {
    pub id: Uuid,
    pub name: String,
    pub callsign: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub member_count: i64,
}

/// Club member enriched with participant presence data.
#[derive(Debug, Clone, FromRow)]
pub struct EnrichedClubMember {
    pub callsign: String,
    pub role: String,
    pub joined_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub last_grid: Option<String>,
    pub is_carrier_wave_user: bool,
}

/// Activity row with the originating callsign (for club activity feeds).
#[derive(Debug, Clone, FromRow)]
pub struct ActivityWithCallsign {
    pub id: Uuid,
    pub callsign: String,
    pub user_id: Uuid,
    pub activity_type: String,
    pub timestamp: DateTime<Utc>,
    pub details: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Admin operations
// ---------------------------------------------------------------------------

/// Create a new club and return the inserted row.
pub async fn create_club(
    pool: &PgPool,
    name: &str,
    callsign: Option<&str>,
    description: Option<&str>,
) -> Result<Club, AppError> {
    let club = sqlx::query_as::<_, Club>(
        r#"
        INSERT INTO clubs (name, callsign, description)
        VALUES ($1, $2, $3)
        RETURNING id, name, callsign, description, created_at, updated_at
        "#,
    )
    .bind(name)
    .bind(callsign)
    .bind(description)
    .fetch_one(pool)
    .await?;

    Ok(club)
}

/// Update a club's metadata.
///
/// Uses COALESCE so only provided fields are updated -- `None` means "keep existing value".
/// This means optional fields (callsign, description) cannot be explicitly cleared to NULL
/// through this function. If clearing is needed, add a dedicated endpoint.
pub async fn update_club(
    pool: &PgPool,
    club_id: Uuid,
    name: Option<&str>,
    callsign: Option<&str>,
    description: Option<&str>,
) -> Result<Option<Club>, AppError> {
    let club = sqlx::query_as::<_, Club>(
        r#"
        UPDATE clubs
        SET name        = COALESCE($2, name),
            callsign    = COALESCE($3, callsign),
            description = COALESCE($4, description),
            updated_at  = now()
        WHERE id = $1
        RETURNING id, name, callsign, description, created_at, updated_at
        "#,
    )
    .bind(club_id)
    .bind(name)
    .bind(callsign)
    .bind(description)
    .fetch_optional(pool)
    .await?;

    Ok(club)
}

/// Delete a club by ID. Returns true if a row was deleted.
pub async fn delete_club(pool: &PgPool, club_id: Uuid) -> Result<bool, AppError> {
    let result = sqlx::query("DELETE FROM clubs WHERE id = $1")
        .bind(club_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Add (or upsert) members to a club. Each entry is (callsign, role).
/// On conflict the role is updated.
pub async fn add_members(
    pool: &PgPool,
    club_id: Uuid,
    members: &[(String, String)],
) -> Result<Vec<ClubMember>, AppError> {
    if members.is_empty() {
        return Ok(vec![]);
    }

    let mut inserted = Vec::with_capacity(members.len());

    for (callsign, role) in members {
        let member = sqlx::query_as::<_, ClubMember>(
            r#"
            INSERT INTO club_members (club_id, callsign, role)
            VALUES ($1, $2, $3)
            ON CONFLICT (club_id, callsign)
            DO UPDATE SET role = EXCLUDED.role
            RETURNING id, club_id, callsign, role, joined_at
            "#,
        )
        .bind(club_id)
        .bind(callsign.to_uppercase())
        .bind(role)
        .fetch_one(pool)
        .await?;

        inserted.push(member);
    }

    Ok(inserted)
}

/// Remove a member from a club. Returns true if a row was deleted.
pub async fn remove_member(
    pool: &PgPool,
    club_id: Uuid,
    callsign: &str,
) -> Result<bool, AppError> {
    let result = sqlx::query(
        r#"
        DELETE FROM club_members
        WHERE club_id = $1 AND callsign = $2
        "#,
    )
    .bind(club_id)
    .bind(callsign.to_uppercase())
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Update a member's role. Returns true if the row was updated.
pub async fn update_member_role(
    pool: &PgPool,
    club_id: Uuid,
    callsign: &str,
    role: &str,
) -> Result<bool, AppError> {
    let result = sqlx::query(
        r#"
        UPDATE club_members
        SET role = $3
        WHERE club_id = $1 AND callsign = $2
        "#,
    )
    .bind(club_id)
    .bind(callsign.to_uppercase())
    .bind(role)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

// ---------------------------------------------------------------------------
// Authenticated queries
// ---------------------------------------------------------------------------

/// Get all clubs that a callsign belongs to, with member counts.
pub async fn get_clubs_for_callsign(
    pool: &PgPool,
    callsign: &str,
) -> Result<Vec<ClubWithCount>, AppError> {
    let clubs = sqlx::query_as::<_, ClubWithCount>(
        r#"
        SELECT c.id, c.name, c.callsign, c.description,
               c.created_at, c.updated_at,
               (SELECT COUNT(*) FROM club_members cm2
                WHERE cm2.club_id = c.id) AS member_count
        FROM clubs c
        JOIN club_members cm ON cm.club_id = c.id
        WHERE cm.callsign = $1
        ORDER BY c.name
        "#,
    )
    .bind(callsign.to_uppercase())
    .fetch_all(pool)
    .await?;

    Ok(clubs)
}

/// Get a single club by ID.
pub async fn get_club_detail(
    pool: &PgPool,
    club_id: Uuid,
) -> Result<Option<Club>, AppError> {
    let club = sqlx::query_as::<_, Club>(
        r#"
        SELECT id, name, callsign, description, created_at, updated_at
        FROM clubs
        WHERE id = $1
        "#,
    )
    .bind(club_id)
    .fetch_optional(pool)
    .await?;

    Ok(club)
}

/// Get club members enriched with participant presence data.
/// LEFT JOINs participants for last_seen_at and Carrier Wave user detection.
pub async fn get_club_members_enriched(
    pool: &PgPool,
    club_id: Uuid,
) -> Result<Vec<EnrichedClubMember>, AppError> {
    let members = sqlx::query_as::<_, EnrichedClubMember>(
        r#"
        SELECT cm.callsign,
               cm.role,
               cm.joined_at,
               p.last_seen_at,
               CAST(NULL AS TEXT) AS last_grid,
               (p.id IS NOT NULL) AS is_carrier_wave_user
        FROM club_members cm
        LEFT JOIN participants p ON UPPER(p.callsign) = cm.callsign
        WHERE cm.club_id = $1
        ORDER BY cm.role = 'admin' DESC, cm.callsign
        "#,
    )
    .bind(club_id)
    .fetch_all(pool)
    .await?;

    Ok(members)
}

/// Get activities from club members, cursor-paginated by created_at DESC.
pub async fn get_club_activity(
    pool: &PgPool,
    club_id: Uuid,
    cursor: Option<DateTime<Utc>>,
    limit: i64,
) -> Result<Vec<ActivityWithCallsign>, AppError> {
    let limit = limit.min(100).max(1);

    let rows = if let Some(before) = cursor {
        sqlx::query_as::<_, ActivityWithCallsign>(
            r#"
            SELECT a.id, a.callsign, a.user_id, a.activity_type,
                   a.timestamp, a.details, a.created_at
            FROM activities a
            JOIN club_members cm ON cm.callsign = UPPER(a.callsign)
            WHERE cm.club_id = $1
              AND a.created_at < $2
            ORDER BY a.created_at DESC
            LIMIT $3
            "#,
        )
        .bind(club_id)
        .bind(before)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, ActivityWithCallsign>(
            r#"
            SELECT a.id, a.callsign, a.user_id, a.activity_type,
                   a.timestamp, a.details, a.created_at
            FROM activities a
            JOIN club_members cm ON cm.callsign = UPPER(a.callsign)
            WHERE cm.club_id = $1
            ORDER BY a.created_at DESC
            LIMIT $2
            "#,
        )
        .bind(club_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    Ok(rows)
}

/// Check whether a callsign is a member of a given club.
pub async fn is_club_member(
    pool: &PgPool,
    club_id: Uuid,
    callsign: &str,
) -> Result<bool, AppError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM club_members WHERE club_id = $1 AND callsign = $2)",
    )
    .bind(club_id)
    .bind(callsign.to_uppercase())
    .fetch_one(pool)
    .await?;

    Ok(exists)
}
