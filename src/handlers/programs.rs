use axum::extract::State;
use sqlx::PgPool;

use crate::db;
use crate::error::AppError;
use crate::extractors::{Json, Path};
use crate::models::{ProgramListResponse, ProgramResponse};

use super::DataResponse;

/// GET /v1/programs — list all active programs.
pub async fn list_programs(
    State(pool): State<PgPool>,
) -> Result<Json<DataResponse<ProgramListResponse>>, AppError> {
    let programs = db::list_programs(&pool).await?;
    let version = db::get_programs_version(&pool).await?;

    let response = ProgramListResponse {
        programs: programs.into_iter().map(ProgramResponse::from).collect(),
        version,
    };

    Ok(Json(DataResponse { data: response }))
}

/// GET /v1/programs/:slug — get a single program by slug.
pub async fn get_program(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
) -> Result<Json<DataResponse<ProgramResponse>>, AppError> {
    let program = db::get_program(&pool, &slug)
        .await?
        .ok_or(AppError::ProgramNotFound { slug })?;

    Ok(Json(DataResponse {
        data: program.into(),
    }))
}
