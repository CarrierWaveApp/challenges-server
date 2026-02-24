# Challenges Server

> **IMPORTANT:** For general project context, read this file and linked docs.
> Only explore source files when actively implementing, planning, or debugging.

## Overview

A self-hostable Rust/Axum HTTP API server for ham radio challenge tracking. Enables operators to track progress toward awards (DXCC, WAS, POTA milestones) with leaderboards and time-limited competitions. The official FullDuplex challenges server is one deployment of this codebase.

## Quick Reference

| Area | Description | Details |
|------|-------------|---------|
| Architecture | Server structure, database, auth flow | [docs/architecture.md](docs/architecture.md) |
| API | Public and admin endpoints | [docs/api.md](docs/api.md) |
| Challenges | Challenge types, scoring, tiers | [docs/features/challenges.md](docs/features/challenges.md) |
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

**NEVER build, run tests, or run the server yourself. Always prompt the user to do so.**

When you need to verify changes compile or tests pass, ask the user to run the appropriate command and report back the results.

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
| Migrations | [docs/index/migrations.md](docs/index/migrations.md) |
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
- `DELETE /v1/activities/{id}` - Delete own activity (auth required)
- `GET /v1/health` - Health check
- `POST /v1/admin/challenges` - Create challenge (admin)
- `PUT /v1/admin/challenges/{id}` - Update challenge (admin)
- `DELETE /v1/admin/challenges/{id}` - Delete challenge (admin)

### Not Yet Implemented
- Badge upload/retrieval
- Invite token generation
- Challenge snapshots
- Rate limiting middleware
- Token revocation endpoint
