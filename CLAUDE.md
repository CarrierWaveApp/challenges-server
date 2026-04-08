# Challenges Server

> **IMPORTANT:** For general project context, read this file and linked docs.
> Only explore source files when actively implementing, planning, or debugging.

## Overview

A self-hostable Rust/Axum HTTP API server for ham radio challenge tracking. Enables operators to track progress toward awards (DXCC, WAS, POTA milestones) with leaderboards and time-limited competitions. The official FullDuplex challenges server is one deployment of this codebase.

## Terminology

- **"admin app"** refers to the **iOS admin app** in `ios-admin/`, NOT the web UI in `web/`.
- The web UI (`web/`) is built into the server binary and served as static files.

## Quick Reference

| Area | Description | Details |
|------|-------------|---------|
| Architecture | Server structure, database, auth flow | [docs/architecture.md](docs/architecture.md) |
| API | Public and admin endpoints | [docs/api.md](docs/api.md) |
| Challenges | Challenge types, scoring, tiers | [docs/features/challenges.md](docs/features/challenges.md) |
| Contest Definitions | Declarative JSON format for contests | [docs/features/contest-definitions.md](docs/features/contest-definitions.md) |
| Leaderboards | Ranking, snapshots, queries | [docs/features/leaderboards.md](docs/features/leaderboards.md) |
| Auth | Device tokens, callsign verification | [docs/features/auth.md](docs/features/auth.md) |
| File Index | Source file locations and exports | [docs/index/](docs/index/) |

## Code Standards

- **Maximum file size: 1000 lines.** Refactor when approaching this limit.
- **Update file index on changes.** When adding, removing, or modifying file exports, update the corresponding `docs/index/*.md` file.
- Use `thiserror` for error types with `IntoResponse` impl
- All database queries via `sqlx` with compile-time checking
- Handlers return `Result<Json<T>, AppError>`
- Configuration via environment variables only

## Building and Running

You may build and run tests. To minimize token usage, always pipe build/test output through `tail` to capture only the final result lines, and use `2>&1` to merge stderr. Never run the server (`cargo run`) yourself.

### Commands

```bash
# Development
cargo build                    # Build the project
cargo test                     # Run tests
cargo run                      # Run server (requires DATABASE_URL)

# Database
sqlx database create           # Create database
sqlx migrate run               # Run migrations

# Docker
docker compose up -d           # Start Postgres + server
docker compose down            # Stop services
```

### Environment Variables

```bash
DATABASE_URL=postgres://user:pass@localhost:5432/challenges  # Required
ADMIN_TOKEN=your-secret-token                                 # Required
PORT=8080                                                     # Optional, default 8080
BASE_URL=https://challenges.example.com                       # Optional
RUST_LOG=info                                                 # Optional
RBN_PROXY_ENABLED=false                                       # Optional, default false
RBN_PROXY_CALLSIGN=W6JSV                                      # Optional, default W6JSV
SNAPSHOT_ENABLED=true                                          # Optional, default true
SNAPSHOT_DIR=data/snapshots                                    # Optional, default data/snapshots
SNAPSHOT_INTERVAL_HOURS=1                                      # Optional, default 1
SNAPSHOT_MAX_AGE_HOURS=24                                      # Optional, default 24
```

## Finding Code

**Use the file index to locate code. Do not use Glob or find commands.**

| Area | Index File |
|------|------------|
| HTTP handlers | [docs/index/handlers.md](docs/index/handlers.md) |
| Database queries | [docs/index/db.md](docs/index/db.md) |
| Data structures | [docs/index/models.md](docs/index/models.md) |
| Authentication | [docs/index/auth.md](docs/index/auth.md) |
| Core (main, config, error) | [docs/index/core.md](docs/index/core.md) |
| Contest Definitions (loader/validator) | [docs/index/contest.md](docs/index/contest.md) |
| Migrations | [docs/index/migrations.md](docs/index/migrations.md) |
| POTA Stats | [docs/index/pota_stats.md](docs/index/pota_stats.md) |
| Park Boundaries | [docs/index/park_boundaries.md](docs/index/park_boundaries.md) |
| Historic Trails | [docs/index/historic_trails.md](docs/index/historic_trails.md) |
| RBN Proxy | [docs/index/rbn.md](docs/index/rbn.md) |
| Tests | [docs/index/tests.md](docs/index/tests.md) |

**Search policy:**
1. Consult the relevant index file first
2. Use Grep for specific symbol searches only if the index doesn't help
3. If you need broader file discovery, ask the user for permission before using Glob or find

## Implementation Status

### Completed
- Database schema with migrations
- Configuration from environment variables
- Error types with JSON responses
- Models for challenges, participants, progress
- Device token generation and validation
- Auth middleware (optional, required, admin)
- Database layer for all entities
- HTTP handlers for all public endpoints
- HTTP handlers for admin CRUD
- Router with middleware stack
- Health check endpoint

### Endpoints Implemented
- `GET /v1/challenges` - List challenges
- `GET /v1/challenges/{id}` - Get challenge details
- `POST /v1/challenges/{id}/join` - Join challenge
- `POST /v1/challenges/{id}/progress` - Report progress (auth required)
- `GET /v1/challenges/{id}/progress` - Get own progress (auth required)
- `GET /v1/challenges/{id}/leaderboard` - Get leaderboard
- `DELETE /v1/challenges/{id}/leave` - Leave challenge (auth required)
- `GET /v1/programs` - List active programs
- `GET /v1/programs/{slug}` - Get program by slug
- `GET /v1/equipment/catalog` - Equipment catalog with ETag and optional `since` delta
- `GET /v1/equipment/search` - Fuzzy equipment search (q, category, limit)
- `PUT /v1/account/callsign` - Change callsign across all tables (auth required)
- `DELETE /v1/activities/{id}` - Delete own activity (auth required)
- `GET /v1/pota/stats/activator` - Activator stats with rank
- `GET /v1/pota/stats/hunter` - Hunter stats with rank
- `GET /v1/pota/stats/state/:state` - State aggregate stats
- `GET /v1/pota/stats/park/:reference` - Park detail with stats
- `GET /v1/pota/stats/rankings/activators` - Paginated activator leaderboard
- `GET /v1/pota/stats/status` - Sync progress and completion status
- `GET /v1/parks/boundaries` - Park boundary polygons (by refs or bbox)
- `GET /v1/parks/boundaries/{reference}` - Single park boundary (full resolution)
- `GET /v1/trails` - Historic trail lines (by refs or bbox)
- `GET /v1/trails/{reference}` - Single trail (full resolution)
- `GET /v1/trails/status` - Trail sync progress and completion status
- `GET /v1/rbn/spots` - RBN spots with filters (call, band, mode, freq range, spotter, since)
- `GET /v1/rbn/stats` - RBN aggregate statistics (band/mode breakdown, rate)
- `GET /v1/rbn/skimmers` - Active RBN skimmers with spot counts
- `GET /v1/health` - Health check (includes RBN status when enabled)
- `GET /v1/events` - List approved events near a location
- `GET /v1/events/{id}` - Get single approved event
- `POST /v1/events` - Submit a new event (auth required)
- `PUT /v1/events/{id}` - Edit own event (auth required)
- `DELETE /v1/events/{id}` - Delete own event (auth required)
- `GET /v1/events/mine` - List own submitted events (auth required)
- `GET /v1/clubs/sync` - Batch-fetch all clubs with members and ETag support (auth required)
- `GET /v1/clubs/{id}/logo` - Serve club logo image (public, no auth)
- `POST /v1/telemetry/upload-errors` - Report anonymized upload error telemetry (auth required)
- `GET /v1/admin/telemetry/upload-errors` - Upload error telemetry summary (admin)
- `POST /v1/admin/challenges` - Create challenge (admin)
- `PUT /v1/admin/challenges/{id}` - Update challenge (admin)
- `DELETE /v1/admin/challenges/{id}` - Delete challenge (admin)
- `POST /v1/admin/clubs/{id}/import-notes` - Import members from callsign notes URL (admin)
- `PUT /v1/admin/clubs/{id}/logo` - Upload or replace club logo (admin)
- `DELETE /v1/admin/clubs/{id}/logo` - Remove club logo (admin)
- `GET /v1/admin/trails/status` - Historic trails sync status (admin)
- `POST /v1/admin/equipment` - Create equipment entry (admin)
- `PUT /v1/admin/equipment/{id}` - Update equipment entry (admin)
- `DELETE /v1/admin/equipment/{id}` - Delete equipment entry (admin)
- `GET /v1/admin/events` - List events with status filter (admin)
- `GET /v1/admin/events/{id}` - Get any event regardless of status (admin)
- `PUT /v1/admin/events/{id}` - Edit any event (admin)
- `PUT /v1/admin/events/{id}/review` - Approve or reject event (admin)
- `DELETE /v1/admin/events/{id}` - Delete any event (admin)
- `GET /v1/admin/events/submitter/{callsign}` - Get submitter history (admin)

### Not Yet Implemented
- Badge upload/retrieval
- Invite token generation
- Challenge snapshots
- Rate limiting middleware
- Token revocation endpoint
