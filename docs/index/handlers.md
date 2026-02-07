# Handlers Index

HTTP request handlers for all API endpoints.

## Files

### `src/handlers/mod.rs`
Module declarations and re-exports for all handlers.

**Exports:**
- Re-exports all public items from submodules

### `src/handlers/challenges.rs`
Challenge CRUD operations and listing.

**Exports:**
- `struct DataResponse<T>` - Generic wrapper for JSON responses with `data` field
- `struct ListChallengesResponse` - Paginated challenge list response
- `async fn list_challenges()` - GET /v1/challenges - List challenges with filtering
- `async fn get_challenge()` - GET /v1/challenges/:id - Get challenge details with ETag
- `async fn create_challenge()` - POST /v1/admin/challenges - Create new challenge (admin)
- `async fn update_challenge()` - PUT /v1/admin/challenges/:id - Update challenge (admin)
- `async fn delete_challenge()` - DELETE /v1/admin/challenges/:id - Delete challenge (admin)

### `src/handlers/join.rs`
Challenge participation management.

**Exports:**
- `async fn join_challenge()` - POST /v1/challenges/:id/join - Join a challenge
- `async fn leave_challenge()` - DELETE /v1/challenges/:id/leave - Leave a challenge (auth required)

### `src/handlers/progress.rs`
Progress reporting and score calculation.

**Exports:**
- `async fn report_progress()` - POST /v1/challenges/:id/progress - Report progress (auth required)
- `async fn get_progress()` - GET /v1/challenges/:id/progress - Get own progress (auth required)
- `fn calculate_score()` - Calculate score based on challenge config
- `fn calculate_percentage()` - Calculate completion percentage
- `fn calculate_percentage_from_progress()` - Calculate percentage from stored progress
- `fn get_total_goals()` - Get total goal count from config
- `fn determine_tier()` - Determine current tier based on score

### `src/handlers/leaderboard.rs`
Leaderboard queries.

**Exports:**
- `async fn get_leaderboard()` - GET /v1/challenges/:id/leaderboard - Get leaderboard with pagination

### `src/handlers/participants.rs`
Participant queries with callsign-based authorization.

**Exports:**
- `async fn get_participation_status()` - GET /v1/challenges/:id/participants/:callsign - Get participation status (auth required, callsign must match)
- `async fn list_challenges_for_callsign()` - GET /v1/participants/:callsign/challenges - List all challenges for a callsign (auth required, callsign must match)

### `src/handlers/health.rs`
Health check endpoint.

**Exports:**
- `struct HealthResponse` - Health check response with status and version
- `async fn health_check()` - GET /v1/health - Return server health status

### `src/handlers/badges.rs`
Badge upload, listing, and retrieval.

**Exports:**
- `struct BadgeListResponse` - List of badges for a challenge
- `async fn upload_badge()` - POST /v1/admin/challenges/:id/badges - Upload badge image (admin)
- `async fn list_badges()` - GET /v1/admin/challenges/:id/badges - List badges (admin)
- `async fn get_badge_image()` - GET /v1/badges/:id/image - Get badge image data
- `async fn delete_badge()` - DELETE /v1/admin/badges/:id - Delete badge (admin)

### `src/handlers/invites.rs`
Invite token management.

**Exports:**
- `struct InviteListResponse` - List of invites for a challenge
- `async fn generate_invite()` - POST /v1/admin/challenges/:id/invites - Generate invite token (admin)
- `async fn list_invites()` - GET /v1/admin/challenges/:id/invites - List invites (admin)
- `async fn revoke_invite()` - DELETE /v1/admin/invites/:token - Revoke invite token (admin)

### `src/handlers/invite_page.rs`
Server-rendered HTML page for friend invite links opened in browsers.

**Exports:**
- `async fn invite_page()` - GET /invite/:token - Render HTML page with inviter callsign and deep link to Carrier Wave

### `src/handlers/friends.rs`
Friend invite links and friend requests.

**Exports:**
- `async fn get_invite_link()` - GET /v1/friends/invite-link - Generate friend invite link (auth required)
- `async fn create_friend_request()` - POST /v1/friends/requests - Create friend request by user ID or invite token (auth required)
