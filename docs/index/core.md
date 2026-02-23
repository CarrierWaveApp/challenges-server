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
