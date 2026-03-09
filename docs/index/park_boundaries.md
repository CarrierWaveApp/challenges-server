# Park Boundaries Index

POTA park boundary polygon cache: fetches from PAD-US ArcGIS API, caches in PostGIS, serves GeoJSON.

## Files

### `migrations/011_park_boundaries.sql`
Database schema for park boundary cache.

**Tables:**
- `park_boundaries` - Cached boundary polygons (pota_reference PK, PostGIS geometry columns, match metadata)

**Indexes:**
- `idx_park_boundaries_geom` - GIST index on full-resolution geometry
- `idx_park_boundaries_geom_simple` - GIST index on pre-simplified geometry

### `src/models/park_boundary.rs`
Data structures for park boundaries feature.

**Query params:**
- `struct BoundariesQuery` - refs, bbox, limit, simplify parameters

**DB row types:**
- `struct ParkBoundaryRow` - Row from park_boundaries with geometry as GeoJSON string

**API response types:**
- `struct BoundaryFeature` - GeoJSON Feature with properties
- `struct BoundaryProperties` - reference, name, designation, manager, acreage, match_quality, source
- `struct BoundariesResponse` - GeoJSON FeatureCollection with meta
- `struct BoundariesMeta` - matched count and unmatched_refs list

**WFS API types (GDOŚ Poland):**
- `struct WfsFeatureCollection` - Response from GDOŚ WFS GetFeature
- `struct WfsFeature` - Single feature with properties and geometry
- `struct WfsProperties` - GDOŚ field mapping (nazwa, area_ha, inspire_id)

**ArcGIS API types (upstream JSON):**
- `struct ArcGisResponse` - Response from PAD-US FeatureServer query
- `struct ArcGisFeature` - Single feature with attributes and geometry
- `struct ArcGisAttributes` - PAD-US field mapping (Loc_Nm, Mang_Name, Des_Tp, GIS_Acres, etc.)

### `src/db/park_boundaries.rs`
Database queries for park boundaries.

**API support:**
- `async fn get_boundaries_by_refs()` - Batch lookup by POTA references (simplified geometry)
- `async fn get_boundary_by_ref()` - Single park lookup (full-resolution geometry)
- `async fn get_boundaries_by_bbox()` - Spatial query with pre-simplified geometry
- `async fn get_boundaries_by_bbox_simplified()` - Spatial query with custom simplification tolerance

**Aggregator support:**
- `async fn upsert_boundary()` - Insert/update boundary with PostGIS geometry conversion
- `async fn count_boundaries()` - Count total cached boundaries
- `async fn get_unfetched_parks()` - Get US parks without cached boundaries
- `async fn get_unfetched_polish_parks()` - Get SP- parks without cached boundaries
- `async fn get_stale_boundaries()` - Get boundaries older than N days for refresh

**Helper types:**
- `struct UnfetchedPark` - Park needing boundary fetch (reference, name, location, lat/lon)
- `struct StaleBoundary` - Stale boundary needing refresh

### `src/aggregators/park_boundaries.rs`
Background aggregator that fetches park boundaries from PAD-US ArcGIS API.

**Exports:**
- `struct ParkBoundariesConfig` - batch_size, cycle_hours, stale_days
- `async fn poll_loop()` - Main loop: fetch unfetched parks, refresh stale, sleep

**Internal functions:**
- `async fn fetch_boundary()` - Fetch boundary for single park (name match, then spatial fallback)
- `async fn query_by_name()` - Query PAD-US by name + state
- `async fn query_by_point()` - Query PAD-US by point intersection
- `async fn save_feature()` - Save ArcGIS feature to database
- `fn normalize_park_name()` - Strip common suffixes for search
- `fn is_federal_park()` - Determine federal vs state service URL
- `fn state_code_to_name()` - Convert US-XX to full state name

### `src/aggregators/polish_park_boundaries.rs`
Background aggregator that fetches Polish park boundaries from GDOŚ WFS.

**Exports:**
- `struct PolishParkBoundariesConfig` - batch_size, cycle_hours, stale_days
- `async fn poll_loop()` - Main loop: fetch unfetched SP- parks, refresh stale, sleep

**Internal functions:**
- `async fn fetch_boundary()` - Fetch boundary for single Polish park
- `async fn query_wfs_by_name()` - Query GDOŚ WFS by name using CQL filter
- `async fn query_wfs_by_point()` - Query GDOŚ WFS by BBOX intersection
- `async fn save_wfs_feature()` - Save WFS feature to database
- `fn normalize_polish_park_name()` - Strip Polish park name suffixes for search
- `fn urlencoded()` - URL-encode for WFS query parameters

**WFS Layers queried:**
- `ParkiNarodowe` (National Parks)
- `ParkiKrajobrazowe` (Landscape Parks)
- `Rezerwaty` (Nature Reserves)
- `ObszaryChronionegoKrajobrazu` (Protected Landscape Areas)
- `ObszarySpecjalnejOchrony` (Special Protection Areas)
- `SpecjalneObszaryOchrony` (Special Areas of Conservation)

### `src/handlers/park_boundaries.rs`
HTTP handlers for park boundaries API endpoints.

**Exports:**
- `async fn get_boundaries()` - GET /v1/parks/boundaries?refs=...&bbox=...
- `async fn get_boundary()` - GET /v1/parks/boundaries/:reference
