# Core Index

Entry point, configuration, and error handling.

## Files

### `src/main.rs`
Application entry point and router setup.

**Exports:**
- `async fn main()` - Initialize tracing, load config, connect to database, run migrations, start server
- `fn create_router()` - Build Axum router with all routes and middleware

**Route Groups:**
- Public routes (optional auth): `/v1/challenges`, `/v1/challenges/:id`, `/v1/challenges/:id/join`, `/v1/challenges/:id/leaderboard`, `/v1/badges/:id/image`, `/v1/programs`, `/v1/programs/:slug`, `/v1/health`
- Authenticated routes (require auth): `/v1/challenges/:id/progress`, `/v1/challenges/:id/leave`, `/v1/friends/invite-link`, `/v1/friends/requests`
- Admin routes (require admin token): `/v1/admin/challenges`, `/v1/admin/challenges/:id`, `/v1/admin/challenges/:id/badges`, `/v1/admin/badges/:id`, `/v1/admin/challenges/:id/invites`, `/v1/admin/invites/:token`
- Metrics: `/metrics` - Prometheus text exposition format
- Static files: Fallback to `web/dist/` with SPA routing support

### `src/config.rs`
Environment variable configuration.

**Exports:**
- `struct Config` - Application configuration with database_url, admin_token, port, base_url, invite_base_url, invite_expiry_days
- `impl Config::from_env()` - Load config from environment variables
- `enum ConfigError` - Configuration errors (Missing, Invalid)

**Environment Variables:**
- `DATABASE_URL` - Required, Postgres connection string
- `ADMIN_TOKEN` - Required, admin API authentication
- `PORT` - Optional, default 8080
- `BASE_URL` - Optional, for generating URLs
- `INVITE_BASE_URL` - Optional, default "https://activities.carrierwave.app", base URL for friend invite links
- `INVITE_EXPIRY_DAYS` - Optional, default 7, how long friend invite links are valid

### `src/metrics.rs`
Prometheus metrics: definitions, middleware, and exposition endpoint.

**Exports:**
- `fn init_metrics()` - Install Prometheus recorder, return handle
- `fn render_metrics()` - Render all metrics in Prometheus text format
- `async fn track_http_metrics()` - Axum middleware for request count, duration, in-flight gauge
- `async fn metrics_handler()` - GET /metrics endpoint handler
- `fn record_auth_validation()` - Record token validation outcome
- `fn record_token_issued()` - Record new device token issuance
- `fn record_db_pool_stats()` - Record DB pool active/idle/size gauges
- `fn record_rbn_connected()` - Record RBN connection state
- `fn record_rbn_spots_ingested()` - Record spots ingested count
- `fn record_rbn_store_size()` - Record current RBN store size
- `fn record_rbn_parse_error()` - Record RBN parse error
- `fn record_aggregator_sync_duration()` - Record aggregator sync duration histogram
- `fn record_aggregator_records_synced()` - Record records synced counter
- `fn record_aggregator_error()` - Record aggregator error counter
- `fn record_unique_client()` - Record unique client seen

**Metrics Exported:**
- `http_requests_total` - Counter by method, path, status
- `http_request_duration_seconds` - Histogram by method, path, status
- `http_requests_in_flight` - Gauge by method, path
- `auth_token_validations_total` - Counter by outcome
- `auth_tokens_issued_total` - Counter
- `db_pool_connections_active` - Gauge
- `db_pool_connections_idle` - Gauge
- `db_pool_size` - Gauge
- `rbn_connected` - Gauge (0/1)
- `rbn_spots_ingested_total` - Counter
- `rbn_store_size` - Gauge
- `rbn_parse_errors_total` - Counter
- `aggregator_sync_duration_seconds` - Histogram by aggregator
- `aggregator_records_synced_total` - Counter by aggregator
- `aggregator_errors_total` - Counter by aggregator
- `unique_clients_seen_total` - Counter
- `process_uptime_seconds` - Gauge

**Tests:**
- `test_normalize_path_uuid` - UUID segments replaced with `:id`
- `test_normalize_path_numeric` - Numeric segments replaced with `:id`
- `test_normalize_path_no_change` - Static paths unchanged
- `test_normalize_path_invite_token` - Invite tokens replaced with `:token`
- `test_normalize_path_named_segment` - Named segments unchanged

### `src/error.rs`
Application error types with HTTP responses.

**Exports:**
- `enum AppError` - All application error variants
- `impl IntoResponse for AppError` - Convert errors to JSON HTTP responses

**Error Variants:**
- `ProgramNotFound` - 404, slug in details
- `ChallengeNotFound` - 404, challenge_id in details
- `BadgeNotFound` - 404, badge_id in details
- `InviteNotFound` - 404, token in details
- `UserNotFound` - 404, user_id in details
- `FriendInviteNotFound` - 404, token in details (expired or not found)
- `FriendInviteUsed` - 410 Gone, token in details
- `AlreadyJoined` - 409 Conflict
- `AlreadyFriends` - 409 Conflict
- `FriendRequestExists` - 409 Conflict
- `CannotFriendSelf` - 422 Unprocessable Entity
- `NotParticipating` - 403 Forbidden
- `InviteRequired` - 403 Forbidden
- `InviteExpired` - 403 Forbidden
- `InviteExhausted` - 403 Forbidden
- `MaxParticipants` - 403 Forbidden
- `ChallengeEnded` - 400 Bad Request
- `InvalidToken` - 401 Unauthorized
- `RateLimited` - 429 Too Many Requests
- `Validation` - 400 Bad Request with message
- `Database` - 500 Internal (from sqlx::Error)
- `Internal` - 500 Internal with message
