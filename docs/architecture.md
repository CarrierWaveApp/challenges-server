# Architecture

## System Overview

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  FullDuplex     │     │  Challenges     │     │  PostgreSQL     │
│  iOS App        │────▶│  Server (Axum)  │────▶│  Database       │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                               │
                               ▼
                        ┌─────────────────┐
                        │  Web            │
                        │  Configurator   │
                        │  (separate)     │
                        └─────────────────┘
```

The server is stateless - all state lives in PostgreSQL. Multiple server instances can run behind a load balancer.

## Key Components

### Request Flow

1. Request arrives at Axum router
2. Middleware stack processes request:
   - CORS headers
   - Tracing/logging
   - Rate limiting (per IP)
   - Auth extraction (device token → callsign)
3. Handler executes business logic
4. Response with rate limit headers

### Module Responsibilities

| Module | Responsibility |
|--------|----------------|
| `config` | Parse environment variables into typed config |
| `error` | `AppError` enum with `IntoResponse` for consistent error format |
| `auth/middleware` | Extract device token, validate, attach callsign to request |
| `auth/token` | Generate secure device tokens |
| `db/*` | All database queries, one file per entity |
| `models/*` | Request/response structs, database row types |
| `handlers/*` | HTTP handlers, one file per endpoint group |
| `scoring/calculator` | Score calculation for different scoring methods |
| `middleware/rate_limit` | Per-IP rate limiting with Redis-like in-memory store |
| `middleware/admin_auth` | Validate admin token for protected endpoints |
| `rbn/store` | In-memory RBN spot store (1-hour VecDeque with RwLock) |
| `rbn/ingester` | RBN telnet stream connection, parsing, batch ingestion |

## Database Schema

See [migrations/](../migrations/) for full schema. Key tables:

| Table | Purpose |
|-------|---------|
| `challenges` | Challenge definitions with JSONB configuration |
| `participants` | Callsigns and their device tokens |
| `challenge_participants` | Join table: who's in which challenge |
| `progress` | Current progress per callsign per challenge |
| `badges` | Badge images (stored as BYTEA) |
| `earned_badges` | Which callsigns earned which badges |
| `challenge_snapshots` | Frozen leaderboards for ended challenges |
| `invite_tokens` | Invite codes for private challenges |
| `park_boundaries` | Cached POTA park boundary polygons (PostGIS geometry) |

### Key Indexes

- `idx_participants_callsign` - Fast callsign lookup
- `idx_progress_leaderboard` - `(challenge_id, score DESC)` for ranking queries

## Authentication Model

No user accounts. Identity is callsign + device token.

1. First join: server generates `fd_{32 random chars}` token
2. Client stores token in Keychain
3. Subsequent requests include `Authorization: Bearer fd_xxx`
4. Server maps token → callsign for authorization

Multiple devices per callsign supported (each gets own token).

## Configuration

All configuration via environment variables (12-factor app):

| Variable | Required | Description |
|----------|----------|-------------|
| `DATABASE_URL` | Yes | Postgres connection string |
| `ADMIN_TOKEN` | Yes | Secret for admin endpoints |
| `PORT` | No | HTTP port (default 8080) |
| `BASE_URL` | No | Public URL for invite links |
| `RUST_LOG` | No | Log level (default info) |

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `axum` | 0.7 | HTTP framework |
| `sqlx` | 0.8 | Async Postgres with compile-time checks (PostGIS for park boundaries) |
| `tokio` | 1 | Async runtime |
| `serde` | 1 | JSON serialization |
| `tower-http` | 0.5 | CORS, tracing middleware |
| `uuid` | 1 | ID generation |
| `chrono` | 0.4 | Timestamps |
| `thiserror` | 1 | Error types |
| `tracing` | 0.1 | Structured logging |

## Deployment

### Docker

Primary deployment method. See `docker-compose.yml` for development setup.

```bash
docker run -e DATABASE_URL=... -e ADMIN_TOKEN=... fullduplex/challenges
```

### Binary

Can also run as standalone binary after `cargo build --release`.

## Self-Hosting

Anyone can deploy their own instance. The server is designed to be:

- **Stateless**: No local file storage (badges in DB)
- **Configurable**: All settings via env vars
- **Isolated**: No phone-home or federation (yet)

Community servers appear in the iOS app when users add the server URL.
