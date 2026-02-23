# Migrations Index

SQL migrations for database schema and seed data.

## Files

### `migrations/001_initial_schema.sql`
Initial database schema with all tables and indexes.

**Tables:**
- `challenges` - Challenge definitions with configuration JSON
  - Columns: id, version, name, description, author, category, challenge_type, configuration, invite_config, hamalert_config, is_active, created_at, updated_at
  - Constraints: category IN (award, event, club, personal, other), challenge_type IN (collection, cumulative, timeBounded)

- `participants` - User accounts with device tokens
  - Columns: id, callsign, device_token, device_name, created_at, last_seen_at
  - Indexes: callsign, device_token (unique)

- `challenge_participants` - Many-to-many join table for challenge participation
  - Columns: id, challenge_id, callsign, invite_token, joined_at, status
  - Constraints: status IN (active, left, completed), UNIQUE(challenge_id, callsign)
  - Indexes: challenge_id, callsign

- `progress` - Progress tracking per user per challenge
  - Columns: id, challenge_id, callsign, completed_goals, current_value, score, current_tier, last_qso_date, updated_at
  - Constraints: UNIQUE(challenge_id, callsign)
  - Indexes: (challenge_id, score DESC, updated_at ASC) for leaderboard queries

- `badges` - Badge images stored as binary
  - Columns: id, challenge_id, name, tier_id, image_data, content_type, created_at
  - Indexes: challenge_id

- `earned_badges` - Track which badges users have earned
  - Columns: id, badge_id, callsign, earned_at
  - Constraints: UNIQUE(badge_id, callsign)
  - Indexes: callsign

- `challenge_snapshots` - Frozen leaderboards when challenges end
  - Columns: id, challenge_id, ended_at, final_standings, statistics, created_at
  - Indexes: challenge_id

- `invite_tokens` - Invite codes for private challenges
  - Columns: token (PK), challenge_id, max_uses, use_count, expires_at, created_at
  - Indexes: challenge_id

### `migrations/002_seed_challenges.sql`
Seed data for initial challenges.

**Challenges:**
- `ARRL WAS 250` (id: a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d)
  - Work all 50 US states during 2026
  - Type: collection, Category: award
  - No historical QSOs allowed
  - Time-bounded: 2026-01-01 to 2026-12-31

- `DXCC` (id: b2c3d4e5-f6a7-5b6c-9d0e-1f2a3b4c5d6e)
  - Work 100+ DXCC entities
  - Type: collection, Category: award
  - Historical QSOs allowed
  - Tiers: 50, 100 (DXCC), 150, 200, 250, 300, 331 (Honor Roll)

### `migrations/003_friend_system.sql`
Friend system: users, friend requests, and friend invite links.

**Tables:**
- `users` - Canonical user identity by callsign
  - Columns: id (UUID), callsign (TEXT UNIQUE), created_at
  - Populated from existing participants on migration
  - Indexes: callsign

- `friend_requests` - Friend requests between users
  - Columns: id, from_user_id, to_user_id, status, requested_at, responded_at
  - Constraints: status IN (pending, accepted, declined), UNIQUE(from_user_id, to_user_id)
  - Indexes: from_user_id, to_user_id, status (pending only)

- `friendships` - Bidirectional friendship records
  - Columns: id, user_id, friend_id, created_at
  - Constraints: UNIQUE(user_id, friend_id)
  - Indexes: user_id, friend_id

- `friend_invites` - Friend invite links
  - Columns: id, token, user_id, created_at, expires_at, used_at, used_by_user_id
  - Constraints: token format check (inv_ prefix + 20+ alphanumeric)
  - Indexes: token (unique), user_id, expires_at

### `migrations/004_seed_friends.sql`
Seed data for testing friend system features.

**Test Users:**
- `W1TEST` (id: 550e8400-e29b-41d4-a716-446655440001) - Primary test user
- `W6JSV` (id: 550e8400-e29b-41d4-a716-446655440002) - Friend of W1TEST
- `N3SEED` (id: 550e8400-e29b-41d4-a716-446655440003) - Has pending request to W1TEST
- `AA4DEV` (id: 550e8400-e29b-41d4-a716-446655440004) - Stranger (no relationships)

**Relationships:**
- Friendship: W1TEST ↔ W6JSV
- Pending request: N3SEED → W1TEST
- Active invite: W1TEST's `inv_w1testactiveinvite12345`
- Used invite: `inv_usedinvitetoken1234567` (used by W6JSV)

### `migrations/006_programs.sql`
Activity program registry table and seed data.

**Tables:**
- `programs` - Activity program definitions
  - Columns: slug (PK), name, short_name, icon, icon_url, website, server_base_url, reference_label, reference_format, reference_example, multi_ref_allowed, activation_threshold, supports_rove, capabilities (TEXT[]), adif_my_sig, adif_my_sig_info, adif_sig_field, adif_sig_info_field, data_entry_label, data_entry_placeholder, data_entry_format, sort_order, is_active, created_at, updated_at

**Seed Programs:**
- `casual` (sort 0) - No capabilities
- `pota` (sort 1) - Parks on the Air, full feature set
- `sota` (sort 2) - Summits on the Air, reference + ADIF
- `wwff` (sort 3) - World Wide Flora & Fauna, reference + ADIF
- `iota` (sort 4) - Islands on the Air, reference only
- `lota` (sort 5) - Lighthouses on the Air, reference only
- `aoa` (sort 6) - Agents on Air, reference + hunter + dataEntry + dataVerification
