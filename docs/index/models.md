# Models Index

Data structures for database rows, API requests, and API responses.

## Files

### `src/models/mod.rs`
Module declarations and re-exports for all models.

**Exports:**
- Re-exports all public items from submodules

### `src/models/challenge.rs`
Challenge-related data structures.

**Exports:**
- `struct Challenge` - Database row for challenges table (FromRow)
- `struct ChallengeResponse` - API response for single challenge (Serialize)
- `struct ChallengeListItem` - API response for challenge in list (FromRow, Serialize)
- `struct CreateChallengeRequest` - API request for creating/updating challenge (Deserialize)
- `struct ListChallengesQuery` - Query params for listing challenges (Deserialize)
- `impl From<Challenge> for ChallengeResponse` - Conversion for API response

### `src/models/participant.rs`
Participant and participation data structures.

**Exports:**
- `struct Participant` - Database row for participants table (FromRow)
- `struct ChallengeParticipant` - Database row for challenge_participants table (FromRow)
- `struct JoinChallengeRequest` - API request for joining challenge (Deserialize)
- `struct JoinChallengeResponse` - API response after joining (Serialize)
- `struct ParticipationResponse` - API response for participation status (Serialize)
- `struct ChallengeParticipation` - API response for challenge participation with name (FromRow, Serialize)

### `src/models/progress.rs`
Progress and leaderboard data structures.

**Exports:**
- `struct Progress` - Database row for progress table (FromRow)
- `struct ReportProgressRequest` - API request for reporting progress (Deserialize)
- `struct ProgressResponse` - API response for progress data (Serialize)
- `struct ReportProgressResponse` - API response after reporting progress (Serialize)
- `struct LeaderboardEntry` - Single leaderboard row (FromRow, Serialize)
- `struct LeaderboardResponse` - Full leaderboard response (Serialize)
- `struct LeaderboardQuery` - Query params for leaderboard (Deserialize)

### `src/models/badge.rs`
Badge data structures.

**Exports:**
- `struct Badge` - Database row with image data (FromRow)
- `struct BadgeMetadata` - Database row without image data (FromRow)
- `struct BadgeResponse` - API response for badge (Serialize)
- `struct CreateBadgeFields` - Multipart form fields for badge creation (Deserialize)
- `impl BadgeMetadata::into_response()` - Convert to API response with URL

### `src/models/invite.rs`
Invite token data structures.

**Exports:**
- `struct InviteToken` - Database row for invite_tokens table (FromRow)
- `struct InviteResponse` - API response for invite (Serialize)
- `struct CreateInviteRequest` - API request for creating invite (Deserialize)
- `impl InviteToken::into_response()` - Convert to API response with URL

### `src/models/user.rs`
User data structures.

**Exports:**
- `struct User` - Database row for users table (FromRow)
- `struct UserResponse` - API response for user (Serialize)
- `impl From<User> for UserResponse` - Conversion for API response

### `src/models/friend_request.rs`
Friend request data structures.

**Exports:**
- `struct FriendRequest` - Database row for friend_requests table (FromRow)
- `struct FriendRequestWithCallsigns` - Database row with joined callsigns (FromRow)
- `struct FriendRequestResponse` - API response for friend request (Serialize)
- `struct CreateFriendRequestBody` - API request body with to_user_id or invite_token (Deserialize)
- `struct Friendship` - Database row for friendships table (FromRow)
- `impl From<FriendRequestWithCallsigns> for FriendRequestResponse` - Conversion for API response

### `src/models/program.rs`
Activity program registry data structures.

**Exports:**
- `struct ProgramRow` - Database row for programs table (FromRow)
- `struct ProgramResponse` - API response for single program (Serialize, camelCase)
- `struct AdifFieldMapping` - ADIF field mapping nested object (Serialize)
- `struct DataEntryConfig` - Data entry config nested object (Serialize)
- `struct ProgramListResponse` - API response for program list with version (Serialize)
- `impl From<ProgramRow> for ProgramResponse` - Conversion with ADIF/data-entry flattening

### `src/models/friend_invite.rs`
Friend invite link data structures.

**Exports:**
- `struct FriendInvite` - Database row for friend_invites table (FromRow)
- `struct FriendInviteResponse` - API response for friend invite (Serialize)
- `impl FriendInvite::into_response()` - Convert to API response with URL
