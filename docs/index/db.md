# Database Index

Database query functions using sqlx with compile-time checked queries.

## Files

### `src/db/mod.rs`
Module declarations and re-exports for all database functions.

**Exports:**
- Re-exports all public items from submodules

### `src/db/challenges.rs`
Challenge CRUD queries.

**Exports:**
- `async fn list_challenges()` - List challenges with filtering, returns `(Vec<ChallengeListItem>, i64)`
- `async fn get_challenge()` - Get challenge by ID, returns `Option<Challenge>`
- `async fn create_challenge()` - Insert new challenge, returns `Challenge`
- `async fn update_challenge()` - Update challenge, increments version, returns `Option<Challenge>`
- `async fn delete_challenge()` - Delete challenge by ID, returns `bool`

### `src/db/participants.rs`
Participant and challenge participation management.

**Exports:**
- `async fn get_or_create_participant()` - Get or create participant by callsign, returns `(Participant, bool)`
- `async fn get_participant_by_token()` - Lookup participant by device token, returns `Option<Participant>`
- `async fn join_challenge()` - Create challenge participation, returns `ChallengeParticipant`
- `async fn get_participation()` - Get participation record, returns `Option<ChallengeParticipant>`
- `async fn leave_challenge()` - Set participation status to 'left', returns `bool`
- `async fn revoke_tokens()` - Delete all participant records for callsign, returns `u64`
- `async fn refresh_participant_token()` - Generate and update device token for callsign, returns `Participant`
- `async fn get_challenges_for_callsign()` - Get all active challenge participations for callsign, returns `Vec<ChallengeParticipation>`

### `src/db/progress.rs`
Progress tracking and leaderboard queries.

**Exports:**
- `async fn get_progress()` - Get progress for callsign in challenge, returns `Option<Progress>`
- `async fn upsert_progress()` - Insert or update progress with score/tier, returns `Progress`
- `async fn get_rank()` - Get callsign's rank in challenge, returns `Option<i64>`
- `async fn get_leaderboard()` - Get paginated leaderboard, returns `(Vec<LeaderboardEntry>, i64)`
- `async fn get_leaderboard_around()` - Get leaderboard entries around a callsign, returns `Vec<LeaderboardEntry>`
- `impl From<serde_json::Error> for AppError` - Error conversion

### `src/db/badges.rs`
Badge storage and retrieval.

**Exports:**
- `async fn create_badge()` - Store badge with image data, returns `BadgeMetadata`
- `async fn list_badges()` - List badges for challenge (without image data), returns `Vec<BadgeMetadata>`
- `async fn get_badge()` - Get badge with image data, returns `Option<Badge>`
- `async fn delete_badge()` - Delete badge by ID, returns `bool`

### `src/db/invites.rs`
Invite token management.

**Exports:**
- `fn generate_invite_token()` - Generate random invite token with `inv_` prefix
- `async fn create_invite()` - Create invite token, returns `InviteToken`
- `async fn list_invites()` - List invites for challenge, returns `Vec<InviteToken>`
- `async fn get_invite()` - Get invite by token, returns `Option<InviteToken>`
- `async fn delete_invite()` - Delete invite by token, returns `bool`

### `src/db/users.rs`
User management.

**Exports:**
- `async fn get_user_by_callsign()` - Get user by callsign, returns `Option<User>`
- `async fn get_user_by_id()` - Get user by ID, returns `Option<User>`
- `async fn get_or_create_user()` - Get or create user by callsign, returns `User`

### `src/db/programs.rs`
Program registry queries.

**Exports:**
- `async fn list_programs()` - List active programs ordered by sort_order, returns `Vec<ProgramRow>`
- `async fn get_program()` - Get active program by slug, returns `Option<ProgramRow>`
- `async fn get_programs_version()` - Get max(updated_at) as epoch seconds, returns `i64`

### `src/db/activities.rs`
Activity CRUD queries.

**Exports:**
- `async fn insert_activity()` - Insert new activity, returns `Activity`
- `async fn delete_activity()` - Delete activity by ID with ownership check, returns `()`
- `async fn get_feed_for_user()` - Get activity feed from friends with cursor pagination, returns `Vec<FeedItemRow>`

### `src/db/friend_requests.rs`
Friend request management.

**Exports:**
- `async fn create_friend_request()` - Create friend request, returns `FriendRequestWithCallsigns`
- `async fn get_friend_request()` - Get request by ID, returns `Option<FriendRequest>`
- `async fn get_pending_request_between()` - Check for pending request between users, returns `Option<FriendRequest>`
- `async fn are_friends()` - Check if users are friends, returns `bool`
- `async fn accept_friend_request()` - Accept request and create friendships, returns `Option<FriendRequestWithCallsigns>`
- `async fn decline_friend_request()` - Decline request, returns `Option<FriendRequest>`

### `src/db/friend_invites.rs`
Friend invite link management.

**Exports:**
- `async fn create_friend_invite()` - Create friend invite, returns `FriendInvite`
- `async fn get_friend_invite()` - Get invite by token, returns `Option<FriendInvite>`
- `async fn get_valid_friend_invite()` - Get valid (not expired, not used) invite, returns `Option<FriendInvite>`
- `async fn mark_invite_used()` - Mark invite as used, returns `Option<FriendInvite>`
- `async fn cleanup_expired_invites()` - Delete old expired/used invites, returns `u64`
