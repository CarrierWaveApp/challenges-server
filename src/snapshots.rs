// src/snapshots.rs
//
//! Periodic disk snapshots of aggregated data (parks, GIS, statistics).
//!
//! Writes JSON files to a configurable directory every N hours so that state
//! can be recovered on startup when the data is less than a configurable max
//! age. This avoids expensive re-fetches from upstream APIs after restarts.

use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::fs;

use crate::config::Config;

// ---------------------------------------------------------------------------
// Snapshot envelope
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotManifest {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub pota_parks_count: usize,
    pub pota_activations_count: usize,
    pub pota_hunter_qsos_count: usize,
    pub pota_fetch_status_count: usize,
    pub park_boundaries_count: usize,
    pub historic_trails_count: usize,
}

// ---------------------------------------------------------------------------
// Serializable row types (mirrors of DB rows)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ParkSnapshot {
    pub reference: String,
    pub name: String,
    pub location_desc: Option<String>,
    pub state: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub grid: Option<String>,
    pub active: bool,
    pub total_attempts: i32,
    pub total_activations: i32,
    pub total_qsos: i32,
    pub stats_fetched_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ActivationSnapshot {
    pub park_reference: String,
    pub callsign: String,
    pub qso_date: NaiveDate,
    pub total_qsos: i32,
    pub qsos_cw: i32,
    pub qsos_data: i32,
    pub qsos_phone: i32,
    pub state: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct HunterQsoSnapshot {
    pub park_reference: String,
    pub callsign: String,
    pub qso_count: i32,
    pub state: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct FetchStatusSnapshot {
    pub park_reference: String,
    pub activations_fetched_at: Option<DateTime<Utc>>,
    pub leaderboard_fetched_at: Option<DateTime<Utc>>,
    pub fetch_error: Option<String>,
    pub consecutive_errors: i32,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct BoundarySnapshot {
    pub pota_reference: String,
    pub park_name: String,
    pub designation: Option<String>,
    pub manager: Option<String>,
    pub acreage: Option<f64>,
    pub match_quality: String,
    pub source: String,
    pub geometry_json: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct TrailSnapshot {
    pub trail_reference: String,
    pub trail_name: String,
    pub designation: Option<String>,
    pub managing_agency: Option<String>,
    pub length_miles: Option<f64>,
    pub state: Option<String>,
    pub match_quality: String,
    pub source: String,
    pub geometry_json: Option<String>,
}

// ---------------------------------------------------------------------------
// Export queries
// ---------------------------------------------------------------------------

async fn export_parks(pool: &PgPool) -> Result<Vec<ParkSnapshot>, sqlx::Error> {
    sqlx::query_as::<_, ParkSnapshot>(
        r#"SELECT reference, name, location_desc, state, latitude, longitude,
                  grid, active, total_attempts, total_activations, total_qsos,
                  stats_fetched_at
           FROM pota_parks"#,
    )
    .fetch_all(pool)
    .await
}

async fn export_activations(pool: &PgPool) -> Result<Vec<ActivationSnapshot>, sqlx::Error> {
    sqlx::query_as::<_, ActivationSnapshot>(
        r#"SELECT park_reference, callsign, qso_date, total_qsos,
                  qsos_cw, qsos_data, qsos_phone, state
           FROM pota_activations"#,
    )
    .fetch_all(pool)
    .await
}

async fn export_hunter_qsos(pool: &PgPool) -> Result<Vec<HunterQsoSnapshot>, sqlx::Error> {
    sqlx::query_as::<_, HunterQsoSnapshot>(
        r#"SELECT park_reference, callsign, qso_count, state
           FROM pota_hunter_qsos"#,
    )
    .fetch_all(pool)
    .await
}

async fn export_fetch_status(pool: &PgPool) -> Result<Vec<FetchStatusSnapshot>, sqlx::Error> {
    sqlx::query_as::<_, FetchStatusSnapshot>(
        r#"SELECT park_reference, activations_fetched_at, leaderboard_fetched_at,
                  fetch_error, consecutive_errors
           FROM pota_fetch_status"#,
    )
    .fetch_all(pool)
    .await
}

async fn export_boundaries(pool: &PgPool) -> Result<Vec<BoundarySnapshot>, sqlx::Error> {
    sqlx::query_as::<_, BoundarySnapshot>(
        r#"SELECT pota_reference, park_name, designation, manager, acreage,
                  match_quality, source,
                  ST_AsGeoJSON(geometry) as geometry_json
           FROM park_boundaries"#,
    )
    .fetch_all(pool)
    .await
}

async fn export_trails(pool: &PgPool) -> Result<Vec<TrailSnapshot>, sqlx::Error> {
    sqlx::query_as::<_, TrailSnapshot>(
        r#"SELECT trail_reference, trail_name, designation, managing_agency,
                  length_miles, state, match_quality, source,
                  ST_AsGeoJSON(geometry) as geometry_json
           FROM historic_trails"#,
    )
    .fetch_all(pool)
    .await
}

// ---------------------------------------------------------------------------
// Import queries
// ---------------------------------------------------------------------------

async fn import_parks(pool: &PgPool, parks: &[ParkSnapshot]) -> Result<u64, sqlx::Error> {
    let mut count = 0u64;
    for p in parks {
        sqlx::query(
            r#"INSERT INTO pota_parks
                   (reference, name, location_desc, state, latitude, longitude,
                    grid, active, total_attempts, total_activations, total_qsos,
                    stats_fetched_at)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
               ON CONFLICT (reference) DO NOTHING"#,
        )
        .bind(&p.reference)
        .bind(&p.name)
        .bind(&p.location_desc)
        .bind(&p.state)
        .bind(p.latitude)
        .bind(p.longitude)
        .bind(&p.grid)
        .bind(p.active)
        .bind(p.total_attempts)
        .bind(p.total_activations)
        .bind(p.total_qsos)
        .bind(p.stats_fetched_at)
        .execute(pool)
        .await?;
        count += 1;
    }
    Ok(count)
}

async fn import_activations(
    pool: &PgPool,
    rows: &[ActivationSnapshot],
) -> Result<u64, sqlx::Error> {
    let mut count = 0u64;
    for r in rows {
        sqlx::query(
            r#"INSERT INTO pota_activations
                   (park_reference, callsign, qso_date, total_qsos,
                    qsos_cw, qsos_data, qsos_phone, state)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
               ON CONFLICT (park_reference, callsign, qso_date) DO NOTHING"#,
        )
        .bind(&r.park_reference)
        .bind(&r.callsign)
        .bind(r.qso_date)
        .bind(r.total_qsos)
        .bind(r.qsos_cw)
        .bind(r.qsos_data)
        .bind(r.qsos_phone)
        .bind(&r.state)
        .execute(pool)
        .await?;
        count += 1;
    }
    Ok(count)
}

async fn import_hunter_qsos(
    pool: &PgPool,
    rows: &[HunterQsoSnapshot],
) -> Result<u64, sqlx::Error> {
    let mut count = 0u64;
    for r in rows {
        sqlx::query(
            r#"INSERT INTO pota_hunter_qsos (park_reference, callsign, qso_count, state)
               VALUES ($1,$2,$3,$4)
               ON CONFLICT (park_reference, callsign) DO NOTHING"#,
        )
        .bind(&r.park_reference)
        .bind(&r.callsign)
        .bind(r.qso_count)
        .bind(&r.state)
        .execute(pool)
        .await?;
        count += 1;
    }
    Ok(count)
}

async fn import_fetch_status(
    pool: &PgPool,
    rows: &[FetchStatusSnapshot],
) -> Result<u64, sqlx::Error> {
    let mut count = 0u64;
    for r in rows {
        sqlx::query(
            r#"INSERT INTO pota_fetch_status
                   (park_reference, activations_fetched_at, leaderboard_fetched_at,
                    fetch_error, consecutive_errors)
               VALUES ($1,$2,$3,$4,$5)
               ON CONFLICT (park_reference) DO NOTHING"#,
        )
        .bind(&r.park_reference)
        .bind(r.activations_fetched_at)
        .bind(r.leaderboard_fetched_at)
        .bind(&r.fetch_error)
        .bind(r.consecutive_errors)
        .execute(pool)
        .await?;
        count += 1;
    }
    Ok(count)
}

async fn import_boundaries(
    pool: &PgPool,
    rows: &[BoundarySnapshot],
) -> Result<u64, sqlx::Error> {
    let mut count = 0u64;
    for r in rows {
        if let Some(ref geojson) = r.geometry_json {
            sqlx::query(
                r#"INSERT INTO park_boundaries
                       (pota_reference, park_name, designation, manager, acreage,
                        match_quality, geometry, geometry_simplified, source,
                        fetched_at, matched_at)
                   VALUES ($1,$2,$3,$4,$5,$6,
                           ST_Multi(ST_CollectionExtract(ST_GeomFromGeoJSON($7), 3)),
                           ST_Simplify(ST_Multi(ST_CollectionExtract(ST_GeomFromGeoJSON($7), 3)), 0.001),
                           $8, NOW(), NOW())
                   ON CONFLICT (pota_reference) DO NOTHING"#,
            )
            .bind(&r.pota_reference)
            .bind(&r.park_name)
            .bind(&r.designation)
            .bind(&r.manager)
            .bind(r.acreage)
            .bind(&r.match_quality)
            .bind(geojson)
            .bind(&r.source)
            .execute(pool)
            .await?;
        } else {
            // no-match sentinel row
            sqlx::query(
                r#"INSERT INTO park_boundaries
                       (pota_reference, park_name, match_quality, geometry,
                        geometry_simplified, source, fetched_at, matched_at)
                   VALUES ($1,$2,$3, NULL, NULL, $4, NOW(), NOW())
                   ON CONFLICT (pota_reference) DO NOTHING"#,
            )
            .bind(&r.pota_reference)
            .bind(&r.park_name)
            .bind(&r.match_quality)
            .bind(&r.source)
            .execute(pool)
            .await?;
        }
        count += 1;
    }
    Ok(count)
}

async fn import_trails(pool: &PgPool, rows: &[TrailSnapshot]) -> Result<u64, sqlx::Error> {
    let mut count = 0u64;
    for r in rows {
        if let Some(ref geojson) = r.geometry_json {
            sqlx::query(
                r#"INSERT INTO historic_trails
                       (trail_reference, trail_name, designation, managing_agency,
                        length_miles, state, match_quality,
                        geometry, geometry_simplified, source,
                        fetched_at, matched_at)
                   VALUES ($1,$2,$3,$4,$5,$6,$7,
                           ST_Multi(ST_CollectionExtract(ST_GeomFromGeoJSON($8), 2)),
                           ST_Simplify(ST_Multi(ST_CollectionExtract(ST_GeomFromGeoJSON($8), 2)), 0.005),
                           $9, NOW(), NOW())
                   ON CONFLICT (trail_reference) DO NOTHING"#,
            )
            .bind(&r.trail_reference)
            .bind(&r.trail_name)
            .bind(&r.designation)
            .bind(&r.managing_agency)
            .bind(r.length_miles)
            .bind(&r.state)
            .bind(&r.match_quality)
            .bind(geojson)
            .bind(&r.source)
            .execute(pool)
            .await?;
        }
        count += 1;
    }
    Ok(count)
}

// ---------------------------------------------------------------------------
// Snapshot save
// ---------------------------------------------------------------------------

/// Write an atomic JSON file (write to .tmp, then rename).
async fn write_json<T: Serialize>(dir: &Path, filename: &str, data: &T) -> std::io::Result<()> {
    let path = dir.join(filename);
    let tmp = dir.join(format!("{filename}.tmp"));
    let json = serde_json::to_string(data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write(&tmp, json.as_bytes()).await?;
    fs::rename(&tmp, &path).await?;
    Ok(())
}

/// Read a JSON file and deserialize it.
async fn read_json<T: serde::de::DeserializeOwned>(dir: &Path, filename: &str) -> Option<T> {
    let path = dir.join(filename);
    let bytes = fs::read(&path).await.ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Save a full snapshot to disk.
pub async fn save_snapshot(pool: &PgPool, dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir)
        .await
        .map_err(|e| format!("create snapshot dir: {e}"))?;

    let parks = export_parks(pool)
        .await
        .map_err(|e| format!("export parks: {e}"))?;
    let activations = export_activations(pool)
        .await
        .map_err(|e| format!("export activations: {e}"))?;
    let hunter_qsos = export_hunter_qsos(pool)
        .await
        .map_err(|e| format!("export hunter_qsos: {e}"))?;
    let fetch_status = export_fetch_status(pool)
        .await
        .map_err(|e| format!("export fetch_status: {e}"))?;
    let boundaries = export_boundaries(pool)
        .await
        .map_err(|e| format!("export boundaries: {e}"))?;
    let trails = export_trails(pool)
        .await
        .map_err(|e| format!("export trails: {e}"))?;

    let manifest = SnapshotManifest {
        version: 1,
        created_at: Utc::now(),
        pota_parks_count: parks.len(),
        pota_activations_count: activations.len(),
        pota_hunter_qsos_count: hunter_qsos.len(),
        pota_fetch_status_count: fetch_status.len(),
        park_boundaries_count: boundaries.len(),
        historic_trails_count: trails.len(),
    };

    write_json(dir, "manifest.json", &manifest)
        .await
        .map_err(|e| format!("write manifest: {e}"))?;
    write_json(dir, "pota_parks.json", &parks)
        .await
        .map_err(|e| format!("write parks: {e}"))?;
    write_json(dir, "pota_activations.json", &activations)
        .await
        .map_err(|e| format!("write activations: {e}"))?;
    write_json(dir, "pota_hunter_qsos.json", &hunter_qsos)
        .await
        .map_err(|e| format!("write hunter_qsos: {e}"))?;
    write_json(dir, "pota_fetch_status.json", &fetch_status)
        .await
        .map_err(|e| format!("write fetch_status: {e}"))?;
    write_json(dir, "park_boundaries.json", &boundaries)
        .await
        .map_err(|e| format!("write boundaries: {e}"))?;
    write_json(dir, "historic_trails.json", &trails)
        .await
        .map_err(|e| format!("write trails: {e}"))?;

    tracing::info!(
        parks = parks.len(),
        activations = activations.len(),
        hunter_qsos = hunter_qsos.len(),
        boundaries = boundaries.len(),
        trails = trails.len(),
        "Snapshot saved to {}",
        dir.display()
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Snapshot restore
// ---------------------------------------------------------------------------

/// Check if tables are empty (candidate for restore).
async fn tables_are_empty(pool: &PgPool) -> Result<bool, sqlx::Error> {
    let park_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pota_parks")
        .fetch_one(pool)
        .await?;
    let boundary_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM park_boundaries")
        .fetch_one(pool)
        .await?;
    let trail_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM historic_trails")
        .fetch_one(pool)
        .await?;
    Ok(park_count.0 == 0 && boundary_count.0 == 0 && trail_count.0 == 0)
}

/// Restore from the most recent snapshot if all target tables are empty and
/// the snapshot is within the configured max age.
pub async fn try_restore(pool: &PgPool, config: &Config) -> Result<bool, String> {
    let dir = PathBuf::from(&config.snapshot_dir);

    let manifest: SnapshotManifest = match read_json(&dir, "manifest.json").await {
        Some(m) => m,
        None => {
            tracing::info!("No snapshot manifest found, skipping restore");
            return Ok(false);
        }
    };

    // Check age
    let age = Utc::now() - manifest.created_at;
    let max_age = chrono::Duration::hours(config.snapshot_max_age_hours as i64);
    if age > max_age {
        tracing::info!(
            age_hours = age.num_hours(),
            max_hours = config.snapshot_max_age_hours,
            "Snapshot too old, skipping restore"
        );
        return Ok(false);
    }

    // Check if tables are empty
    let empty = tables_are_empty(pool)
        .await
        .map_err(|e| format!("check tables empty: {e}"))?;
    if !empty {
        tracing::info!("Tables already have data, skipping snapshot restore");
        return Ok(false);
    }

    tracing::info!(
        created_at = %manifest.created_at,
        age_hours = age.num_hours(),
        "Restoring from snapshot"
    );

    // Restore parks first (foreign key target)
    if let Some(parks) = read_json::<Vec<ParkSnapshot>>(&dir, "pota_parks.json").await {
        let n = import_parks(pool, &parks)
            .await
            .map_err(|e| format!("import parks: {e}"))?;
        tracing::info!(count = n, "Restored pota_parks");
    }

    if let Some(rows) = read_json::<Vec<FetchStatusSnapshot>>(&dir, "pota_fetch_status.json").await
    {
        let n = import_fetch_status(pool, &rows)
            .await
            .map_err(|e| format!("import fetch_status: {e}"))?;
        tracing::info!(count = n, "Restored pota_fetch_status");
    }

    if let Some(rows) = read_json::<Vec<ActivationSnapshot>>(&dir, "pota_activations.json").await {
        let n = import_activations(pool, &rows)
            .await
            .map_err(|e| format!("import activations: {e}"))?;
        tracing::info!(count = n, "Restored pota_activations");
    }

    if let Some(rows) = read_json::<Vec<HunterQsoSnapshot>>(&dir, "pota_hunter_qsos.json").await {
        let n = import_hunter_qsos(pool, &rows)
            .await
            .map_err(|e| format!("import hunter_qsos: {e}"))?;
        tracing::info!(count = n, "Restored pota_hunter_qsos");
    }

    if let Some(rows) = read_json::<Vec<BoundarySnapshot>>(&dir, "park_boundaries.json").await {
        let n = import_boundaries(pool, &rows)
            .await
            .map_err(|e| format!("import boundaries: {e}"))?;
        tracing::info!(count = n, "Restored park_boundaries");
    }

    if let Some(rows) = read_json::<Vec<TrailSnapshot>>(&dir, "historic_trails.json").await {
        let n = import_trails(pool, &rows)
            .await
            .map_err(|e| format!("import trails: {e}"))?;
        tracing::info!(count = n, "Restored historic_trails");
    }

    tracing::info!("Snapshot restore complete");
    Ok(true)
}

// ---------------------------------------------------------------------------
// Background loop
// ---------------------------------------------------------------------------

/// Periodically save snapshots.
pub async fn snapshot_loop(pool: PgPool, config: Config) {
    let dir = PathBuf::from(&config.snapshot_dir);
    let interval = std::time::Duration::from_secs(config.snapshot_interval_hours * 3600);
    let mut ticker = tokio::time::interval(interval);

    // Skip the immediate first tick (let the server start up and aggregators populate data)
    ticker.tick().await;

    loop {
        ticker.tick().await;
        if let Err(e) = save_snapshot(&pool, &dir).await {
            tracing::error!("Snapshot save failed: {e}");
        }
    }
}
