# Implementation Plan: State-Level Park GIS Data Sources

## Goal

Supplement the existing PAD-US boundary fetcher with state-specific GIS data sources to improve match rates for US state parks. Currently, parks that PAD-US doesn't match (or matches poorly) fall through to spatial-only matching or `no_match`. State agencies often publish higher-quality, more current boundary data for their own parks.

## Current Architecture (no changes needed)

The existing system works well:
- `park_boundaries` table with PostGIS geometry, `source` column, `match_quality`
- Background aggregator polls `get_unfetched_parks()`, tries name match then spatial fallback
- `data_source_for_park()` routes by POTA reference prefix
- All sources use the same `save_feature()` / `upsert_boundary()` path

The plan builds on this by adding a **third matching strategy** for US parks: query a state-specific FeatureServer between the PAD-US name match and the PAD-US spatial fallback.

## Data Sources to Add

### Tier 1: States with ArcGIS FeatureServer endpoints (same query pattern as PAD-US)

These can be queried identically to PAD-US — name match + spatial fallback via ArcGIS REST API:

| State | Endpoint | Name Field | Notes |
|-------|----------|------------|-------|
| Florida | `ca.dep.state.fl.us/arcgis/rest/services/OpenData/PARKS_BOUNDARIES/MapServer/8` | TBD | FL DEP maintains boundaries for 175 parks |
| Oregon | `maps.prd.state.or.us/arcgis/rest/services/Land_ownership/Oregon_State_Parks/FeatureServer/0` | TBD | OPRD real property boundaries |
| California | CSP ArcGIS Hub (`csp-public-data-csparks.hub.arcgis.com`) | TBD | Monthly updates, 8 datasets |
| Pennsylvania | PASDA / DCNR FeatureServer | TBD | DCNR State Park Boundaries |

### Tier 2: PAD-US State-Level FeatureServer (already queryable, different URL)

USGS also publishes a **state-level** PAD-US layer that may return different/better results:
- `services.arcgis.com/P3ePLMYs2RVChkJx/arcgis/rest/services/USA_Protected_Areas_State/FeatureServer/0`

This could be tried as an alternative query source for parks that PAD-US Manager_Name layer misses.

### Tier 3: USGS govunits MapServer

- `carto.nationalmap.gov/arcgis/rest/services/govunits/MapServer`
- Contains forest, grassland, park, wilderness, wildlife reserve layers
- Useful as a last-resort fallback for parks not in PAD-US at all

## Implementation Steps

### Step 1: Add state data source registry

**File:** `src/aggregators/park_boundaries.rs`

Add a registry of state-specific ArcGIS endpoints with their field mappings:

```rust
struct StateDataSource {
    state: &'static str,           // "FL", "OR", "CA", "PA"
    url: &'static str,             // ArcGIS FeatureServer URL
    name_field: &'static str,      // Field containing park name
    area_field: Option<&'static str>, // Field containing acreage/area
    out_fields: &'static str,      // Fields to request
    source_label: &'static str,    // e.g. "fl_dep", "or_oprd"
}

const STATE_SOURCES: &[StateDataSource] = &[
    StateDataSource {
        state: "FL",
        url: "https://ca.dep.state.fl.us/arcgis/rest/services/OpenData/PARKS_BOUNDARIES/MapServer/8",
        name_field: "LONG_NAME",       // verify via ?f=pjson
        area_field: Some("SHAPE_Area"),
        out_fields: "LONG_NAME,SHORT_NAME,PARK_TYPE,SHAPE_Area",
        source_label: "fl_dep",
    },
    // ... more states
];
```

**Estimated changes:** ~30 lines in `park_boundaries.rs`

### Step 2: Add state-source query functions

**File:** `src/aggregators/park_boundaries.rs`

Add two new query functions following the existing pattern:

```rust
/// Find the state-specific data source for a park, if one exists.
fn state_source_for_park(reference: &str, location_desc: Option<&str>) -> Option<&'static StateDataSource> {
    let state = location_desc.and_then(state_code_to_abbrev)?;
    STATE_SOURCES.iter().find(|s| s.state == state)
}

/// Query a state-specific FeatureServer by name.
async fn query_state_source_by_name(
    client: &reqwest::Client,
    source: &StateDataSource,
    park_name: &str,
    // ...
) -> Result<Option<ArcGisFeature>, ...>

/// Query a state-specific FeatureServer by point.
async fn query_state_source_by_point(
    client: &reqwest::Client,
    source: &StateDataSource,
    lon: f64,
    lat: f64,
) -> Result<Option<ArcGisFeature>, ...>
```

**Estimated changes:** ~60 lines

### Step 3: Insert state source into the fetch strategy chain

**File:** `src/aggregators/park_boundaries.rs` — modify `fetch_boundary_padus()`

Current flow:
1. PAD-US name + state match → save as "exact"
2. PAD-US spatial fallback → save as "spatial"

New flow:
1. PAD-US name + state match → save as "exact", source="pad_us_4"
2. **State-specific name match** → save as "exact", source="fl_dep" (etc.)
3. **State-specific spatial match** → save as "spatial", source="fl_dep" (etc.)
4. PAD-US spatial fallback → save as "spatial", source="pad_us_4"

This means state sources act as a middle tier — tried when PAD-US name matching fails but before falling back to PAD-US spatial (which is less precise).

**Estimated changes:** ~25 lines modifying `fetch_boundary_padus()`

### Step 4: Discover and validate state FeatureServer endpoints

Before coding, each state endpoint needs manual verification:
1. Hit `{url}?f=pjson` to get field names and geometry type
2. Confirm the name field, area field, and whether it supports `f=geojson`
3. Test a sample query: `{url}/query?where=1=1&resultRecordCount=1&f=geojson`
4. Document which POTA park types each source covers (state parks only? also state forests, WMAs?)

This is research work that should happen before/during Step 1. Start with Florida and Oregon (confirmed ArcGIS endpoints from research).

### Step 5: Add `source` breakdown to status endpoint

**Files:** `src/db/park_boundaries.rs`, `src/models/park_boundary.rs`, `src/handlers/park_boundaries.rs`

Add a `by_source` breakdown to the status response so we can see how many boundaries came from each source:

```rust
// New query
SELECT source, COUNT(*) as count
FROM park_boundaries
WHERE match_quality != 'none'
GROUP BY source

// New response field
pub by_source: HashMap<String, i64>,
```

**Estimated changes:** ~30 lines across 3 files

### Step 6: Update documentation

**Files:** `docs/index/park_boundaries.md`

Add the new state data sources, functions, and config to the index.

## What This Plan Does NOT Include

- **Bulk import of static shapefiles**: Some states only offer downloadable .shp/.gdb files, not queryable APIs. Importing these would require a different pipeline (CLI tool to ingest .shp into PostGIS). This is a separate feature.
- **New migration**: The existing `park_boundaries` table schema is sufficient — the `source` column already accommodates multiple sources.
- **New endpoints**: No new HTTP endpoints needed. Existing `/v1/parks/boundaries` endpoints serve data regardless of source.
- **State-level PAD-US fallback (Tier 2/3)**: Start with Tier 1 only. Tier 2/3 sources can be added later using the same registry pattern.

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| State endpoints change URLs/fields | `source_label` tracks provenance; stale refresh will re-fetch; log warnings on parse failures |
| Rate limiting by state servers | Existing semaphore concurrency limit applies; add per-source delay if needed |
| State data covers fewer park types than PAD-US | State sources are tried *in addition to* PAD-US, not instead of — they only improve matches, never reduce them |
| File size approaching 1000-line limit | `park_boundaries.rs` is currently ~820 lines; state registry + queries add ~115 lines. If it exceeds 1000, extract state sources into `src/aggregators/state_park_sources.rs` |

## Suggested Implementation Order

1. **Step 4** (research) — Verify FL and OR endpoints, document field names
2. **Step 1** — Add `StateDataSource` registry with FL and OR
3. **Step 2** — Add generic state-source query functions
4. **Step 3** — Wire into `fetch_boundary_padus()` strategy chain
5. **Step 5** — Add source breakdown to status
6. **Step 6** — Update docs
7. Build and test
8. Monitor match rates via `/v1/parks/boundaries/status` `by_source` breakdown
9. Add more states to registry as endpoints are discovered and verified
