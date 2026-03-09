use axum::extract::{Query, State};
use sqlx::PgPool;

use crate::db::park_boundaries as db;
use crate::error::AppError;
use crate::extractors::{Json, Path};
use crate::models::park_boundary::{
    BoundariesMeta, BoundariesQuery, BoundariesResponse, BoundaryFeature, BoundaryProperties,
    ParkBoundaryRow,
};

/// GET /v1/parks/boundaries?refs=...&bbox=...&limit=...&simplify=...
pub async fn get_boundaries(
    State(pool): State<PgPool>,
    Query(params): Query<BoundariesQuery>,
) -> Result<Json<BoundariesResponse>, AppError> {
    let limit = params.limit.unwrap_or(50).clamp(1, 100);

    if params.refs.is_none() && params.bbox.is_none() {
        return Err(AppError::Validation {
            message: "Either 'refs' or 'bbox' parameter is required".to_string(),
        });
    }

    // refs query takes priority
    if let Some(ref refs_str) = params.refs {
        let refs: Vec<String> = refs_str
            .split(',')
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty())
            .collect();

        if refs.len() > 20 {
            return Err(AppError::Validation {
                message: "Maximum 20 refs per request".to_string(),
            });
        }

        let rows = db::get_boundaries_by_refs(&pool, &refs).await?;
        let matched_refs: Vec<&str> = rows.iter().map(|r| r.pota_reference.as_str()).collect();
        let unmatched_refs: Vec<String> = refs
            .iter()
            .filter(|r| !matched_refs.contains(&r.as_str()))
            .cloned()
            .collect();

        let features = rows_to_features(rows);
        let matched = features.len();

        return Ok(Json(BoundariesResponse {
            collection_type: "FeatureCollection",
            features,
            meta: BoundariesMeta {
                matched,
                unmatched_refs,
            },
        }));
    }

    // bbox query
    if let Some(ref bbox_str) = params.bbox {
        let coords: Vec<f64> = bbox_str
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();

        if coords.len() != 4 {
            return Err(AppError::Validation {
                message: "bbox must be 'west,south,east,north' (4 numbers)".to_string(),
            });
        }

        let (west, south, east, north) = (coords[0], coords[1], coords[2], coords[3]);

        let rows = if let Some(tolerance) = params.simplify {
            db::get_boundaries_by_bbox_simplified(&pool, west, south, east, north, limit, tolerance)
                .await?
        } else {
            db::get_boundaries_by_bbox(&pool, west, south, east, north, limit).await?
        };

        let features = rows_to_features(rows);
        let matched = features.len();

        return Ok(Json(BoundariesResponse {
            collection_type: "FeatureCollection",
            features,
            meta: BoundariesMeta {
                matched,
                unmatched_refs: vec![],
            },
        }));
    }

    unreachable!()
}

/// GET /v1/parks/boundaries/:reference
pub async fn get_boundary(
    State(pool): State<PgPool>,
    Path(reference): Path<String>,
) -> Result<Json<BoundaryFeature>, AppError> {
    let reference = reference.to_uppercase();

    let row = db::get_boundary_by_ref(&pool, &reference)
        .await?
        .ok_or_else(|| AppError::ParkNotFound {
            reference: reference.clone(),
        })?;

    let feature = row_to_feature(row).ok_or_else(|| AppError::ParkNotFound { reference })?;

    Ok(Json(feature))
}

fn rows_to_features(rows: Vec<ParkBoundaryRow>) -> Vec<BoundaryFeature> {
    rows.into_iter().filter_map(row_to_feature).collect()
}

fn row_to_feature(row: ParkBoundaryRow) -> Option<BoundaryFeature> {
    let geometry_json = row.geometry_json?;
    let geometry: serde_json::Value = serde_json::from_str(&geometry_json).ok()?;

    Some(BoundaryFeature {
        feature_type: "Feature",
        geometry,
        properties: BoundaryProperties {
            reference: row.pota_reference,
            name: row.park_name,
            designation: row.designation,
            manager: row.manager,
            acreage: row.acreage,
            match_quality: row.match_quality,
            source: row.source,
        },
    })
}
