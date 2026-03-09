use sqlx::PgPool;

use crate::models::park_boundary::ParkBoundaryRow;

/// Get boundaries by POTA references, using simplified geometry.
pub async fn get_boundaries_by_refs(
    pool: &PgPool,
    refs: &[String],
) -> Result<Vec<ParkBoundaryRow>, sqlx::Error> {
    sqlx::query_as::<_, ParkBoundaryRow>(
        r#"
        SELECT pota_reference, park_name, designation, manager, acreage,
               match_quality, source,
               ST_AsGeoJSON(geometry_simplified) as geometry_json
        FROM park_boundaries
        WHERE pota_reference = ANY($1)
        "#,
    )
    .bind(refs)
    .fetch_all(pool)
    .await
}

/// Get a single boundary by reference with full-resolution geometry.
pub async fn get_boundary_by_ref(
    pool: &PgPool,
    reference: &str,
) -> Result<Option<ParkBoundaryRow>, sqlx::Error> {
    sqlx::query_as::<_, ParkBoundaryRow>(
        r#"
        SELECT pota_reference, park_name, designation, manager, acreage,
               match_quality, source,
               ST_AsGeoJSON(geometry) as geometry_json
        FROM park_boundaries
        WHERE pota_reference = $1
        "#,
    )
    .bind(reference)
    .fetch_optional(pool)
    .await
}

/// Get boundaries within a bounding box, using simplified geometry.
pub async fn get_boundaries_by_bbox(
    pool: &PgPool,
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    limit: i64,
) -> Result<Vec<ParkBoundaryRow>, sqlx::Error> {
    sqlx::query_as::<_, ParkBoundaryRow>(
        r#"
        SELECT pota_reference, park_name, designation, manager, acreage,
               match_quality, source,
               ST_AsGeoJSON(geometry_simplified) as geometry_json
        FROM park_boundaries
        WHERE geometry_simplified && ST_MakeEnvelope($1, $2, $3, $4, 4326)
        ORDER BY pota_reference
        LIMIT $5
        "#,
    )
    .bind(west)
    .bind(south)
    .bind(east)
    .bind(north)
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Get boundaries within a bounding box with custom simplification tolerance.
pub async fn get_boundaries_by_bbox_simplified(
    pool: &PgPool,
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    limit: i64,
    simplify_tolerance: f64,
) -> Result<Vec<ParkBoundaryRow>, sqlx::Error> {
    sqlx::query_as::<_, ParkBoundaryRow>(
        r#"
        SELECT pota_reference, park_name, designation, manager, acreage,
               match_quality, source,
               ST_AsGeoJSON(ST_Simplify(geometry, $6)) as geometry_json
        FROM park_boundaries
        WHERE geometry_simplified && ST_MakeEnvelope($1, $2, $3, $4, 4326)
        ORDER BY pota_reference
        LIMIT $5
        "#,
    )
    .bind(west)
    .bind(south)
    .bind(east)
    .bind(north)
    .bind(limit)
    .bind(simplify_tolerance)
    .fetch_all(pool)
    .await
}

/// Upsert a park boundary from ArcGIS data.
pub async fn upsert_boundary(
    pool: &PgPool,
    pota_reference: &str,
    park_name: &str,
    designation: Option<&str>,
    manager: Option<&str>,
    acreage: Option<f64>,
    match_quality: &str,
    geojson: &str,
    source: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO park_boundaries
            (pota_reference, park_name, designation, manager, acreage,
             match_quality, geometry, geometry_simplified, source,
             fetched_at, matched_at)
        VALUES ($1, $2, $3, $4, $5, $6,
                ST_Multi(ST_GeomFromGeoJSON($7)),
                ST_Simplify(ST_Multi(ST_GeomFromGeoJSON($7)), 0.001),
                $8, NOW(), NOW())
        ON CONFLICT (pota_reference) DO UPDATE SET
            park_name = EXCLUDED.park_name,
            designation = EXCLUDED.designation,
            manager = EXCLUDED.manager,
            acreage = EXCLUDED.acreage,
            match_quality = EXCLUDED.match_quality,
            geometry = EXCLUDED.geometry,
            geometry_simplified = EXCLUDED.geometry_simplified,
            source = EXCLUDED.source,
            fetched_at = NOW()
        "#,
    )
    .bind(pota_reference)
    .bind(park_name)
    .bind(designation)
    .bind(manager)
    .bind(acreage)
    .bind(match_quality)
    .bind(geojson)
    .bind(source)
    .execute(pool)
    .await?;
    Ok(())
}

/// Count total cached boundaries.
pub async fn count_boundaries(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM park_boundaries")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

/// Get references that need fetching (no boundary row yet), limited to US parks.
pub async fn get_unfetched_parks(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<UnfetchedPark>, sqlx::Error> {
    sqlx::query_as::<_, UnfetchedPark>(
        r#"
        SELECT p.reference, p.name, p.location_desc, p.latitude, p.longitude
        FROM pota_parks p
        LEFT JOIN park_boundaries b ON p.reference = b.pota_reference
        WHERE b.pota_reference IS NULL
          AND p.reference LIKE 'US-%'
          AND p.active = true
        ORDER BY p.reference
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Get stale boundaries (older than given days) for re-fetching.
pub async fn get_stale_boundaries(
    pool: &PgPool,
    stale_days: i64,
    limit: i64,
) -> Result<Vec<StaleBoundary>, sqlx::Error> {
    sqlx::query_as::<_, StaleBoundary>(
        r#"
        SELECT b.pota_reference as reference, p.name, p.location_desc,
               p.latitude, p.longitude
        FROM park_boundaries b
        JOIN pota_parks p ON p.reference = b.pota_reference
        WHERE b.fetched_at < NOW() - make_interval(days => $1::int)
        ORDER BY b.fetched_at ASC
        LIMIT $2
        "#,
    )
    .bind(stale_days)
    .bind(limit)
    .fetch_all(pool)
    .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct UnfetchedPark {
    pub reference: String,
    pub name: String,
    pub location_desc: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct StaleBoundary {
    pub reference: String,
    pub name: String,
    pub location_desc: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}
