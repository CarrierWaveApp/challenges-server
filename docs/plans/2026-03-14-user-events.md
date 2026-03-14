# User-Submitted Events — Implementation Plan

**Date:** 2026-03-14
**PRD:** [docs/features/events.md](../features/events.md)

---

## Overview

Add user-submitted events with admin moderation and proximity-based discovery.

### Scope (this repo)

- **Server**: Migration, models, db layer, handlers, router wiring — all API endpoints (public, authenticated, admin)
- **iOS Admin App** (`ios-admin/`): Event moderation UI — pending events list, review screen, submitter history

### Out of scope (separate repo)

- **CarrierWave iOS App**: Event submission form, map + list discovery view, "My Events" screen, push notifications on approval/rejection. See [PRD](../features/events.md) for full CarrierWave app requirements.

---

## Phase 1: Database Migration

### File: `migrations/019_events.sql`

```sql
CREATE TYPE event_type AS ENUM (
    'club_meeting', 'swap_meet', 'field_day',
    'special_event', 'hamfest', 'net', 'other'
);

CREATE TYPE event_status AS ENUM ('pending', 'approved', 'rejected');

CREATE TABLE events (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL CHECK (char_length(name) <= 200),
    description     TEXT CHECK (char_length(description) <= 2000),
    event_type      event_type NOT NULL,
    start_date      TIMESTAMPTZ NOT NULL,
    end_date        TIMESTAMPTZ,
    timezone        TEXT NOT NULL,
    venue_name      TEXT CHECK (char_length(venue_name) <= 200),
    address         TEXT NOT NULL,
    city            TEXT NOT NULL,
    state           TEXT,
    country         TEXT NOT NULL,
    latitude        DOUBLE PRECISION NOT NULL,
    longitude       DOUBLE PRECISION NOT NULL,
    location        geography(Point, 4326) NOT NULL,
    cost            TEXT CHECK (char_length(cost) <= 100),
    url             TEXT,
    submitted_by    TEXT NOT NULL,
    status          event_status NOT NULL DEFAULT 'pending',
    reviewed_by     TEXT,
    reviewed_at     TIMESTAMPTZ,
    rejection_reason TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Spatial index for proximity queries
CREATE INDEX idx_events_location ON events USING GIST (location);

-- Status + date for listing queries
CREATE INDEX idx_events_status_start ON events (status, start_date);

-- Submitter lookup for "my events"
CREATE INDEX idx_events_submitted_by ON events (submitted_by);
```

The `location` column is auto-populated via a trigger from lat/lon:

```sql
CREATE OR REPLACE FUNCTION events_set_location()
RETURNS TRIGGER AS $$
BEGIN
    NEW.location := ST_SetSRID(ST_MakePoint(NEW.longitude, NEW.latitude), 4326)::geography;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_events_set_location
    BEFORE INSERT OR UPDATE ON events
    FOR EACH ROW
    EXECUTE FUNCTION events_set_location();
```

---

## Phase 2: Models

### File: `src/models/event.rs`

New file. Follows the pattern of `models/challenge.rs`.

**Structs:**

```
EventRow                  — FromRow, full database row
EventResponse             — Serialize, camelCase API response
EventListItem             — FromRow + Serialize, for list queries (excludes description)
CreateEventRequest        — Deserialize, for POST /v1/events
UpdateEventRequest        — Deserialize, for PUT /v1/events/{id} (all fields optional)
AdminUpdateEventRequest   — Deserialize, for PUT /v1/admin/events/{id}
ReviewEventRequest        — Deserialize, { action: "approve"|"reject", reason: Option<String> }
ListEventsQuery           — Deserialize, query params (lat, lon, radius_km, event_type, from_date, to_date, include_past, limit, offset)
AdminListEventsQuery      — Deserialize, query params (status, limit, offset)
MyEventsQuery             — Deserialize, query params (limit, offset)
```

**Conversions:**
- `impl From<EventRow> for EventResponse` — maps DB row to API response with camelCase

**Validation (in handler or impl):**
- `latitude` in range `[-90, 90]`
- `longitude` in range `[-180, 180]`
- `start_date` must be in the future (for new submissions)
- `end_date` must be after `start_date` if provided
- `radius_km` in range `[1, 500]`

**Update to `src/models/mod.rs`:**
- Add `pub mod event;`

---

## Phase 3: Database Layer

### File: `src/db/events.rs`

New file. Follows the pattern of `db/challenges.rs`.

**Functions:**

```rust
// Public queries
async fn list_events_near(pool, lat, lon, radius_km, filters) -> Result<(Vec<EventListItem>, i64)>
async fn get_event(pool, id) -> Result<Option<EventRow>>

// Authenticated queries
async fn create_event(pool, req, callsign) -> Result<EventRow>
async fn update_own_event(pool, id, callsign, req) -> Result<Option<EventRow>>
async fn delete_own_event(pool, id, callsign) -> Result<bool>
async fn list_my_events(pool, callsign, limit, offset) -> Result<Vec<EventListItem>>
async fn count_pending_events(pool, callsign) -> Result<i64>

// Admin queries
async fn list_events_admin(pool, status, limit, offset) -> Result<(Vec<EventListItem>, i64)>
async fn admin_update_event(pool, id, req) -> Result<Option<EventRow>>
async fn review_event(pool, id, action, reviewed_by, reason) -> Result<Option<EventRow>>
async fn admin_delete_event(pool, id) -> Result<bool>
async fn get_submitter_history(pool, callsign) -> Result<SubmitterStats>
```

**Key query: proximity search**

```sql
SELECT id, name, event_type, start_date, end_date, timezone,
       venue_name, city, state, country, latitude, longitude,
       cost, submitted_by, created_at,
       ST_Distance(location, ST_MakePoint($1, $2)::geography) AS distance_meters
FROM events
WHERE status = 'approved'
  AND ($3::bool OR start_date >= NOW())
  AND ST_DWithin(location, ST_MakePoint($1, $2)::geography, $4)
  AND ($5::event_type IS NULL OR event_type = $5)
  AND ($6::timestamptz IS NULL OR start_date >= $6)
  AND ($7::timestamptz IS NULL OR start_date <= $7)
ORDER BY start_date ASC
LIMIT $8 OFFSET $9
```

**Key logic: re-review on key-field edit**

The `update_own_event` function checks if the event was previously `approved` and if any key fields (name, description, address, venue_name, latitude, longitude) changed. If so, it resets `status` to `pending` and clears `reviewed_by`/`reviewed_at`.

**Update to `src/db/mod.rs`:**
- Add `pub mod events;`

---

## Phase 4: Error Variants

### File: `src/error.rs`

Add new variants:

```rust
EventNotFound           // 404 — event_id in details
EventNotOwned           // 403 — cannot edit/delete another user's event
MaxPendingEvents        // 429 — "Maximum pending events reached (10)"
InvalidEventReview      // 400 — invalid action (not "approve" or "reject")
```

---

## Phase 5: Handlers

### File: `src/handlers/events.rs`

New file. Public + authenticated endpoints.

**Functions:**

```rust
async fn list_events()        // GET /v1/events
async fn get_event()          // GET /v1/events/{id}
async fn create_event()       // POST /v1/events (auth required)
async fn update_event()       // PUT /v1/events/{id} (auth required, must own)
async fn delete_event()       // DELETE /v1/events/{id} (auth required, must own)
async fn list_my_events()     // GET /v1/events/mine (auth required)
```

**Validation in `create_event`:**
1. Check `count_pending_events` < 10, else return `MaxPendingEvents`
2. Validate lat/lon ranges
3. Validate start_date is in the future
4. Validate end_date > start_date if provided
5. Insert with status = `pending`

### File: `src/handlers/events_admin.rs`

New file. Admin endpoints.

**Functions:**

```rust
async fn list_events_admin()     // GET /v1/admin/events
async fn admin_update_event()    // PUT /v1/admin/events/{id}
async fn review_event()          // PUT /v1/admin/events/{id}/review
async fn admin_delete_event()    // DELETE /v1/admin/events/{id}
```

**Update to `src/handlers/mod.rs`:**
- Add `pub mod events;`
- Add `pub mod events_admin;`

---

## Phase 6: Router

### File: `src/main.rs`

Add routes to `create_router()`:

```rust
// Public event routes (optional auth)
.route("/v1/events", get(handlers::events::list_events))
.route("/v1/events/{id}", get(handlers::events::get_event))

// Authenticated event routes
.route("/v1/events", post(handlers::events::create_event))
.route("/v1/events/mine", get(handlers::events::list_my_events))
.route("/v1/events/{id}", put(handlers::events::update_event))
.route("/v1/events/{id}", delete(handlers::events::delete_event))

// Admin event routes
.route("/v1/admin/events", get(handlers::events_admin::list_events_admin))
.route("/v1/admin/events/{id}", put(handlers::events_admin::admin_update_event))
.route("/v1/admin/events/{id}/review", put(handlers::events_admin::review_event))
.route("/v1/admin/events/{id}", delete(handlers::events_admin::admin_delete_event))
```

Note: The public `GET /v1/events` and authenticated `POST /v1/events` share the same path but differ by method, so they go on the same `.route()` call:

```rust
.route("/v1/events", get(list_events).post(create_event))
```

Similarly for `GET/PUT/DELETE /v1/events/{id}`.

---

## Phase 7: Documentation Updates

### Update `docs/api.md`
- Add Events section with all endpoint documentation

### Update `docs/index/handlers.md`
- Add `src/handlers/events.rs` and `src/handlers/events_admin.rs` entries

### Update `docs/index/db.md`
- Add `src/db/events.rs` entry

### Update `docs/index/models.md`
- Add `src/models/event.rs` entry

### Update `docs/index/migrations.md`
- Add `migrations/019_events.sql` entry

### Update `CLAUDE.md`
- Add event endpoints to the "Endpoints Implemented" list

---

## Phase 8: Tests

### File: `tests/events_test.rs`

Key test cases:
1. Create event (authenticated) — returns pending status
2. Create event (unauthenticated) — 401
3. List events near location — only returns approved, upcoming events
4. List events with `include_past=true` — includes past events
5. Get single event — only if approved
6. Get pending event — 404 for non-owner
7. Edit own event — success
8. Edit approved event key field — resets to pending
9. Edit approved event non-key field — stays approved
10. Edit someone else's event — 403
11. Delete own event — success
12. Delete someone else's event — 403
13. Max pending events — 429 after 10
14. List my events — shows all statuses
15. Admin list events — filter by status
16. Admin edit event — can edit any event
17. Admin review approve — sets status to approved
18. Admin review reject — sets status to rejected with reason
19. Admin delete — works on any event
20. Proximity query — events outside radius excluded
21. Event type filter — only matching type returned

---

## Implementation Order

1. Migration (`019_events.sql`)
2. Models (`src/models/event.rs`, update `mod.rs`)
3. Error variants (`src/error.rs`)
4. Database layer (`src/db/events.rs`, update `mod.rs`)
5. Handlers (`src/handlers/events.rs`, `events_admin.rs`, update `mod.rs`)
6. Router wiring (`src/main.rs`)
7. Tests (`tests/events_test.rs`)
8. Documentation updates (api.md, index files, CLAUDE.md)

---

## Estimated File Impact

| Action | File |
|--------|------|
| **New** | `migrations/019_events.sql` |
| **New** | `src/models/event.rs` |
| **New** | `src/db/events.rs` |
| **New** | `src/handlers/events.rs` |
| **New** | `src/handlers/events_admin.rs` |
| **New** | `tests/events_test.rs` |
| **Edit** | `src/models/mod.rs` (add module) |
| **Edit** | `src/db/mod.rs` (add module) |
| **Edit** | `src/handlers/mod.rs` (add module) |
| **Edit** | `src/error.rs` (add variants) |
| **Edit** | `src/main.rs` (add routes) |
| **Edit** | `docs/api.md` (add endpoints) |
| **Edit** | `docs/index/handlers.md` |
| **Edit** | `docs/index/db.md` |
| **Edit** | `docs/index/models.md` |
| **Edit** | `docs/index/migrations.md` |
| **Edit** | `CLAUDE.md` (endpoints list) |
