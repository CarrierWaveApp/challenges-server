use axum::{extract::State, http::StatusCode};
use uuid::Uuid;

use crate::extractors::{Json, Path};
use sqlx::PgPool;

use crate::db;
use crate::error::AppError;
use crate::models::club::{
    AddMembersRequest, ClubMemberResponse, ClubResponse, CreateClubRequest, UpdateClubRequest,
    UpdateMemberRoleRequest,
};

use super::DataResponse;

/// GET /v1/admin/clubs
/// List all clubs with member counts.
pub async fn list_clubs_admin(
    State(pool): State<PgPool>,
) -> Result<Json<DataResponse<Vec<ClubResponse>>>, AppError> {
    let clubs = db::clubs::list_all_clubs(&pool).await?;

    let data = clubs
        .into_iter()
        .map(|c| ClubResponse {
            id: c.id,
            name: c.name,
            callsign: c.callsign,
            description: c.description,
            member_count: c.member_count,
        })
        .collect();

    Ok(Json(DataResponse { data }))
}

/// GET /v1/admin/clubs/:id/members
/// List members of a club.
pub async fn list_club_members_admin(
    State(pool): State<PgPool>,
    Path(club_id): Path<Uuid>,
) -> Result<Json<DataResponse<Vec<ClubMemberResponse>>>, AppError> {
    // Verify club exists
    db::clubs::get_club_detail(&pool, club_id)
        .await?
        .ok_or(AppError::ClubNotFound { club_id })?;

    let members = db::clubs::get_club_members_enriched(&pool, club_id).await?;

    let data = members
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

    Ok(Json(DataResponse { data }))
}

/// POST /v1/admin/clubs
/// Create a new club.
pub async fn create_club(
    State(pool): State<PgPool>,
    Json(body): Json<CreateClubRequest>,
) -> Result<(StatusCode, Json<DataResponse<ClubResponse>>), AppError> {
    let club = db::clubs::create_club(
        &pool,
        &body.name,
        body.callsign.as_deref(),
        body.description.as_deref(),
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(DataResponse {
            data: ClubResponse {
                id: club.id,
                name: club.name,
                callsign: club.callsign,
                description: club.description,
                member_count: 0,
            },
        }),
    ))
}

/// PUT /v1/admin/clubs/:id
/// Update a club's metadata.
pub async fn update_club(
    State(pool): State<PgPool>,
    Path(club_id): Path<Uuid>,
    Json(body): Json<UpdateClubRequest>,
) -> Result<Json<DataResponse<ClubResponse>>, AppError> {
    let club = db::clubs::update_club(
        &pool,
        club_id,
        body.name.as_deref(),
        body.callsign.as_deref(),
        body.description.as_deref(),
    )
    .await?
    .ok_or(AppError::ClubNotFound { club_id })?;

    // Fetch current member count
    let members = db::clubs::get_club_members_enriched(&pool, club_id).await?;

    Ok(Json(DataResponse {
        data: ClubResponse {
            id: club.id,
            name: club.name,
            callsign: club.callsign,
            description: club.description,
            member_count: members.len() as i64,
        },
    }))
}

/// DELETE /v1/admin/clubs/:id
/// Delete a club.
pub async fn delete_club(
    State(pool): State<PgPool>,
    Path(club_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let deleted = db::clubs::delete_club(&pool, club_id).await?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::ClubNotFound { club_id })
    }
}

/// POST /v1/admin/clubs/:id/members
/// Add members to a club.
pub async fn add_club_members(
    State(pool): State<PgPool>,
    Path(club_id): Path<Uuid>,
    Json(body): Json<AddMembersRequest>,
) -> Result<(StatusCode, Json<DataResponse<Vec<ClubMemberResponse>>>), AppError> {
    // Verify club exists
    db::clubs::get_club_detail(&pool, club_id)
        .await?
        .ok_or(AppError::ClubNotFound { club_id })?;

    let member_tuples: Vec<(String, String)> = body
        .members
        .into_iter()
        .map(|m| (m.callsign, m.role))
        .collect();

    let members = db::clubs::add_members(&pool, club_id, &member_tuples).await?;

    let data = members
        .into_iter()
        .map(|m| ClubMemberResponse {
            callsign: m.callsign,
            role: m.role,
            joined_at: m.joined_at,
            last_seen_at: None,
            last_grid: None,
            is_carrier_wave_user: false,
        })
        .collect();

    Ok((StatusCode::CREATED, Json(DataResponse { data })))
}

/// DELETE /v1/admin/clubs/:id/members/:callsign
/// Remove a member from a club.
pub async fn remove_club_member(
    State(pool): State<PgPool>,
    Path((club_id, callsign)): Path<(Uuid, String)>,
) -> Result<StatusCode, AppError> {
    let removed = db::clubs::remove_member(&pool, club_id, &callsign).await?;

    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::ClubMemberNotFound { club_id, callsign })
    }
}

/// PUT /v1/admin/clubs/:id/members/:callsign
/// Update a member's role.
pub async fn update_club_member_role(
    State(pool): State<PgPool>,
    Path((club_id, callsign)): Path<(Uuid, String)>,
    Json(body): Json<UpdateMemberRoleRequest>,
) -> Result<Json<DataResponse<ClubMemberResponse>>, AppError> {
    let updated = db::clubs::update_member_role(&pool, club_id, &callsign, &body.role).await?;

    if !updated {
        return Err(AppError::ClubMemberNotFound { club_id, callsign });
    }

    // Fetch the updated member from the enriched view
    let members = db::clubs::get_club_members_enriched(&pool, club_id).await?;
    let callsign_upper = callsign.to_uppercase();
    let member = members
        .into_iter()
        .find(|m| m.callsign == callsign_upper)
        .ok_or(AppError::ClubMemberNotFound {
            club_id,
            callsign: callsign_upper,
        })?;

    Ok(Json(DataResponse {
        data: ClubMemberResponse {
            callsign: member.callsign,
            role: member.role,
            joined_at: member.joined_at,
            last_seen_at: member.last_seen_at,
            last_grid: member.last_grid,
            is_carrier_wave_user: member.is_carrier_wave_user,
        },
    }))
}
