# API Reference

Base URL: `https://challenges.example.com/v1`

## Authentication

### Device Token

Most endpoints require a device token in the Authorization header:

```
Authorization: Bearer fd_abc123...
```

Tokens are issued when joining a challenge and tied to a callsign.

### Admin Token

Admin endpoints require the server's admin token:

```
Authorization: Bearer {ADMIN_TOKEN}
```

## Response Format

### Success

```json
{
  "data": { ... }
}
```

### Error

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable message",
    "details": { ... }
  }
}
```

## Rate Limiting

All responses include rate limit headers:

- `X-RateLimit-Limit`: Requests allowed per window
- `X-RateLimit-Remaining`: Requests remaining
- `X-RateLimit-Reset`: Unix timestamp when window resets

---

## Public Endpoints

### List Challenges

```
GET /v1/challenges
```

**Query Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `category` | string | Filter by category (award, event, club, personal, other) |
| `type` | string | Filter by type (collection, cumulative, timeBounded) |
| `active` | bool | Filter by active status |
| `limit` | int | Max results (default 50, max 100) |
| `offset` | int | Pagination offset |

**Response:**

```json
{
  "data": {
    "challenges": [
      {
        "id": "uuid",
        "name": "Worked All States",
        "description": "Work all 50 US states",
        "category": "award",
        "type": "collection",
        "participantCount": 1234,
        "isActive": true
      }
    ],
    "total": 45,
    "limit": 50,
    "offset": 0
  }
}
```

### Get Challenge

```
GET /v1/challenges/{id}
```

**Response Headers:**

- `ETag`: Version hash for caching
- `X-Challenge-Version`: Integer version number

**Response:**

```json
{
  "data": {
    "id": "uuid",
    "version": 1,
    "name": "Worked All States",
    "description": "...",
    "author": "FullDuplex",
    "category": "award",
    "type": "collection",
    "configuration": {
      "goals": {
        "type": "collection",
        "items": [
          { "id": "US-AL", "name": "Alabama" },
          { "id": "US-AK", "name": "Alaska" }
        ]
      },
      "tiers": [
        { "id": "tier-25", "name": "25 States", "threshold": 25 },
        { "id": "tier-50", "name": "All States", "threshold": 50 }
      ],
      "qualificationCriteria": {
        "bands": null,
        "modes": null,
        "requiredFields": [],
        "dateRange": null,
        "matchRules": [
          { "qsoField": "state", "goalField": "id" }
        ]
      },
      "scoring": {
        "method": "count",
        "displayFormat": "{value}/50 states"
      },
      "historicalQsosAllowed": true
    },
    "badges": [
      { "id": "badge-uuid", "name": "WAS", "tierId": "tier-50" }
    ],
    "isActive": true,
    "createdAt": "2025-01-01T00:00:00Z",
    "updatedAt": "2025-01-01T00:00:00Z"
  }
}
```

### Join Challenge

```
POST /v1/challenges/{id}/join
```

**Request:**

```json
{
  "callsign": "W1ABC",
  "deviceName": "iPhone",
  "inviteToken": "xyz789"
}
```

`inviteToken` only required for invite-only challenges.

**Response:**

```json
{
  "data": {
    "participationId": "uuid",
    "deviceToken": "fd_abc123...",
    "joinedAt": "2025-01-15T12:00:00Z",
    "status": "active",
    "historicalAllowed": true
  }
}
```

**Note:** If the callsign already exists in the system, a new device token is generated and returned. This allows token recovery for users who have lost their token.

**Errors:**

| Code | HTTP | Description |
|------|------|-------------|
| `ALREADY_JOINED` | 409 | Callsign already in challenge |
| `INVITE_REQUIRED` | 403 | Challenge requires invite |
| `INVITE_EXPIRED` | 403 | Invite token expired |
| `INVITE_EXHAUSTED` | 403 | Invite max uses reached |
| `MAX_PARTICIPANTS` | 403 | Challenge full |
| `CHALLENGE_ENDED` | 400 | Time-bounded challenge ended |

### Report Progress

```
POST /v1/challenges/{id}/progress
Authorization: Bearer fd_xxx
```

**Request:**

```json
{
  "completedGoals": ["US-CA", "US-NY", "US-TX"],
  "currentValue": 47,
  "qualifyingQsoCount": 52,
  "lastQsoDate": "2025-01-15T18:30:00Z"
}
```

**Response:**

```json
{
  "data": {
    "accepted": true,
    "serverProgress": {
      "completedGoals": ["US-CA", "US-NY", "US-TX"],
      "currentValue": 47,
      "percentage": 94.0,
      "score": 47,
      "rank": 23,
      "currentTier": "tier-40"
    },
    "newBadges": ["badge-uuid"]
  }
}
```

### Get Progress

```
GET /v1/challenges/{id}/progress
Authorization: Bearer fd_xxx
```

Returns current progress for the authenticated callsign.

### Get Leaderboard

```
GET /v1/challenges/{id}/leaderboard
```

**Query Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `limit` | int | Max results (default 100) |
| `offset` | int | Pagination offset |
| `around` | string | Callsign to center results around |

**Response:**

```json
{
  "data": {
    "leaderboard": [
      {
        "rank": 1,
        "callsign": "K1ABC",
        "score": 50,
        "currentTier": "tier-50",
        "completedAt": "2025-01-10T00:00:00Z"
      }
    ],
    "total": 1234,
    "userPosition": {
      "rank": 23,
      "callsign": "W1ABC",
      "score": 47
    },
    "lastUpdated": "2025-01-15T19:00:00Z"
  }
}
```

### Get Participation Status

```
GET /v1/challenges/{id}/participants/{callsign}
Authorization: Bearer fd_xxx
```

Returns participation status for a callsign in a specific challenge. The authenticated callsign must match the requested callsign.

**Response:**

```json
{
  "data": {
    "participationId": "uuid",
    "challengeId": "uuid",
    "joinedAt": "2025-01-15T12:00:00Z",
    "status": "active"
  }
}
```

**Errors:**

| Code | HTTP | Description |
|------|------|-------------|
| `FORBIDDEN` | 403 | Authenticated callsign doesn't match requested callsign |
| `NOT_PARTICIPATING` | 403 | Callsign is not a participant in this challenge |

### List Challenges for Callsign

```
GET /v1/participants/{callsign}/challenges
Authorization: Bearer fd_xxx
```

Returns all challenges a callsign has joined. The authenticated callsign must match the requested callsign.

**Response:**

```json
{
  "data": [
    {
      "participationId": "uuid",
      "challengeId": "uuid",
      "challengeName": "Worked All States",
      "joinedAt": "2025-01-15T12:00:00Z",
      "status": "active"
    }
  ]
}
```

**Errors:**

| Code | HTTP | Description |
|------|------|-------------|
| `FORBIDDEN` | 403 | Authenticated callsign doesn't match requested callsign |

### Leave Challenge

```
DELETE /v1/challenges/{id}/leave
Authorization: Bearer fd_xxx
```

Removes participation and progress. Cannot be undone.

### Get Snapshot

```
GET /v1/challenges/{id}/snapshot
```

For ended time-bounded challenges, returns frozen final standings.

**Response:**

```json
{
  "data": {
    "challengeId": "uuid",
    "endedAt": "2025-01-31T23:59:59Z",
    "finalStandings": [
      { "rank": 1, "callsign": "K1ABC", "score": 127 }
    ],
    "totalParticipants": 50,
    "statistics": {
      "averageScore": 45.2,
      "completionRate": 0.12
    }
  }
}
```

### Get Badge Image

```
GET /v1/badges/{id}/image
```

Returns badge image with appropriate `Content-Type` header.

### List Programs

```
GET /v1/programs
```

Returns all active activity programs with a version for cache invalidation.

**Response:**

```json
{
  "data": {
    "programs": [
      {
        "slug": "pota",
        "name": "Parks on the Air",
        "shortName": "POTA",
        "icon": "tree",
        "website": "https://pota.app",
        "referenceLabel": "Park Reference",
        "referenceFormat": "^[A-Z]+-[0-9]{4,5}$",
        "referenceExample": "K-0001",
        "multiRefAllowed": true,
        "activationThreshold": 10,
        "supportsRove": true,
        "capabilities": ["referenceField", "adifUpload", "browseSpots", "selfSpot", "hunter", "locationLookup", "progressTracking"],
        "adifFields": {
          "mySig": "POTA",
          "mySigInfo": "MY_POTA_REF",
          "sigField": "SIG",
          "sigInfoField": "SIG_INFO"
        }
      }
    ],
    "version": 1737900000
  }
}
```

### Get Program

```
GET /v1/programs/{slug}
```

Returns a single program by slug.

**Errors:**

| Code | HTTP | Description |
|------|------|-------------|
| `PROGRAM_NOT_FOUND` | 404 | Program slug doesn't exist |

### Delete Activity

```
DELETE /v1/activities/{id}
Authorization: Bearer fd_xxx
```

Deletes an activity. The authenticated user must own the activity.

**Response:** 204 No Content

**Errors:**

| Code | HTTP | Description |
|------|------|-------------|
| `ACTIVITY_NOT_FOUND` | 404 | Activity doesn't exist or not owned |

### Health Check

```
GET /v1/health
```

**Response:**

```json
{
  "status": "ok",
  "version": "1.0.0"
}
```

---

## Event Endpoints

### List Events (Proximity)

```
GET /v1/events
```

Returns approved upcoming events near a location.

**Query Parameters (required):**

| Param | Type | Description |
|-------|------|-------------|
| `lat` | float | Latitude (-90 to 90) |
| `lon` | float | Longitude (-180 to 180) |
| `radiusKm` | float | Search radius in km (1 to 500) |

**Query Parameters (optional):**

| Param | Type | Description |
|-------|------|-------------|
| `eventType` | string | Filter by type |
| `fromDate` | datetime | Events starting on or after |
| `toDate` | datetime | Events starting on or before |
| `includePast` | bool | Include past events (default false) |
| `limit` | int | Max results (default 50, max 100) |
| `offset` | int | Pagination offset |

**Response:**

```json
{
  "data": {
    "events": [
      {
        "id": "uuid",
        "name": "Monthly Club Meeting",
        "eventType": "club_meeting",
        "startDate": "2026-04-01T19:00:00Z",
        "endDate": "2026-04-01T21:00:00Z",
        "timezone": "America/New_York",
        "venueName": "VFW Hall",
        "city": "Springfield",
        "state": "MA",
        "country": "US",
        "latitude": 42.1015,
        "longitude": -72.5898,
        "cost": "Free",
        "submittedBy": "W1ABC",
        "status": "approved",
        "createdAt": "2026-03-14T12:00:00Z",
        "distanceMeters": 1234.56
      }
    ],
    "total": 5,
    "limit": 50,
    "offset": 0
  }
}
```

### Get Event

```
GET /v1/events/{id}
```

Returns a single approved event with full details.

### Submit Event

```
POST /v1/events
Authorization: Bearer fd_xxx
```

Submit a new event for admin review. Max 10 pending events per callsign.

**Request:**

```json
{
  "name": "Monthly Club Meeting",
  "eventType": "club_meeting",
  "startDate": "2026-04-01T19:00:00Z",
  "endDate": "2026-04-01T21:00:00Z",
  "timezone": "America/New_York",
  "venueName": "VFW Hall",
  "address": "123 Main St",
  "city": "Springfield",
  "state": "MA",
  "country": "US",
  "latitude": 42.1015,
  "longitude": -72.5898,
  "cost": "Free",
  "url": "https://example.com/meeting",
  "description": "Monthly meeting of the Springfield ARC",
  "days": [
    {
      "date": "2026-04-01",
      "startTime": "2026-04-01T19:00:00Z",
      "endTime": "2026-04-01T21:00:00Z"
    }
  ]
}
```

The `days` field is optional. When provided, each entry represents a specific day of a multi-day event with its own schedule. The `startDate`/`endDate` fields remain as the overall event span for backward compatibility.

**Response:** 201 Created with the event (status = "pending"). If days were provided, the response includes a `days` array.

**Errors:**

| Code | HTTP | Description |
|------|------|-------------|
| `MAX_PENDING_EVENTS` | 429 | Already have 10 pending events |
| `VALIDATION_ERROR` | 400 | Invalid fields |

### Update Own Event

```
PUT /v1/events/{id}
Authorization: Bearer fd_xxx
```

Edit own event. Partial update — only provided fields change. If the event was approved and key fields (name, description, address, venueName, latitude, longitude) change, status resets to "pending".

### Delete Own Event

```
DELETE /v1/events/{id}
Authorization: Bearer fd_xxx
```

**Response:** 204 No Content

### List My Events

```
GET /v1/events/mine
Authorization: Bearer fd_xxx
```

Returns all events submitted by the authenticated callsign, across all statuses.

---

## Admin Endpoints

All require `Authorization: Bearer {ADMIN_TOKEN}`.

### List Events (Admin)

```
GET /v1/admin/events
```

**Query Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `status` | string | Filter by status (pending, approved, rejected) |
| `limit` | int | Max results (default 50, max 100) |
| `offset` | int | Pagination offset |

### Get Event (Admin)

```
GET /v1/admin/events/{id}
```

Returns any event regardless of status.

### Edit Event (Admin)

```
PUT /v1/admin/events/{id}
```

Admin can edit any event fields (fix typos, adjust coordinates before approving).

### Review Event

```
PUT /v1/admin/events/{id}/review
```

**Request:**

```json
{
  "action": "approve",
  "reason": null
}
```

or

```json
{
  "action": "reject",
  "reason": "Not ham radio related"
}
```

Reason is required when rejecting.

### Delete Event (Admin)

```
DELETE /v1/admin/events/{id}
```

Hard deletes the event.

### Get Submitter History

```
GET /v1/admin/events/submitter/{callsign}
```

Returns submission stats for a callsign.

```json
{
  "data": {
    "totalSubmitted": 12,
    "totalApproved": 9,
    "totalRejected": 2,
    "totalPending": 1
  }
}
```

### Create Challenge

```
POST /v1/admin/challenges
```

**Request:** Full challenge object (see Get Challenge response format).

### Update Challenge

```
PUT /v1/admin/challenges/{id}
```

Increments version number automatically.

### Delete Challenge

```
DELETE /v1/admin/challenges/{id}
```

Cascades to participants, progress, badges.

### Upload Badge

```
POST /v1/admin/challenges/{id}/badges
Content-Type: multipart/form-data
```

**Form Fields:**

- `name`: Badge name
- `tierId`: Associated tier (optional)
- `image`: Image file (PNG, SVG)

### Delete Badge

```
DELETE /v1/admin/badges/{id}
```

### Generate Invite

```
POST /v1/admin/challenges/{id}/invites
```

**Request:**

```json
{
  "maxUses": 50,
  "expiresAt": "2025-12-31T23:59:59Z"
}
```

**Response:**

```json
{
  "data": {
    "token": "invite_abc123",
    "url": "https://challenges.example.com/join/invite_abc123"
  }
}
```

### Revoke Tokens

```
DELETE /v1/admin/participants/{callsign}/tokens
```

Revokes all device tokens for a callsign (abuse handling).

### End Challenge

```
POST /v1/admin/challenges/{id}/end
```

Manually ends a challenge and creates a snapshot.

### List All Programs (Admin)

```
GET /v1/admin/programs
```

Returns all programs including inactive ones.

### Get Program (Admin)

```
GET /v1/admin/programs/{slug}
```

Returns a program by slug, including inactive programs.

### Create Program

```
POST /v1/admin/programs
```

**Request:**

```json
{
  "slug": "my-program",
  "name": "My Program",
  "shortName": "MP",
  "icon": "radio",
  "website": "https://example.com",
  "referenceLabel": "Reference",
  "referenceFormat": "^[A-Z]+-[0-9]{4}$",
  "referenceExample": "K-0001",
  "multiRefAllowed": false,
  "activationThreshold": null,
  "supportsRove": false,
  "capabilities": ["referenceField"],
  "adifMySig": null,
  "adifMySigInfo": null,
  "adifSigField": null,
  "adifSigInfoField": null,
  "dataEntryLabel": null,
  "dataEntryPlaceholder": null,
  "dataEntryFormat": null,
  "sortOrder": 10
}
```

**Response:** 201 Created with the created program.

### Update Program

```
PUT /v1/admin/programs/{slug}
```

Partial update — only provided fields are changed. For nullable fields, send `null` to clear or omit to leave unchanged.

**Request:** Any subset of fields from the create request, plus `isActive`.

### Delete Program

```
DELETE /v1/admin/programs/{slug}
```

Permanently deletes a program. Use `PUT` with `{"isActive": false}` for soft-deactivation.

---

## Error Codes

| Code | HTTP | Description |
|------|------|-------------|
| `ACTIVITY_NOT_FOUND` | 404 | Activity doesn't exist or not owned |
| `CHALLENGE_NOT_FOUND` | 404 | Challenge doesn't exist |
| `PROGRAM_NOT_FOUND` | 404 | Program slug doesn't exist |
| `ALREADY_JOINED` | 409 | Already participating |
| `NOT_PARTICIPATING` | 403 | Must join first |
| `INVITE_REQUIRED` | 403 | Invite-only challenge |
| `INVITE_EXPIRED` | 403 | Invite past expiry |
| `INVITE_EXHAUSTED` | 403 | Invite max uses reached |
| `MAX_PARTICIPANTS` | 403 | Challenge at capacity |
| `CHALLENGE_ENDED` | 400 | Challenge has ended |
| `INVALID_TOKEN` | 401 | Bad or revoked token |
| `FORBIDDEN` | 403 | Access denied (e.g., callsign mismatch) |
| `RATE_LIMITED` | 429 | Too many requests |
| `EVENT_NOT_FOUND` | 404 | Event doesn't exist or not approved |
| `EVENT_NOT_OWNED` | 403 | Cannot modify another user's event |
| `MAX_PENDING_EVENTS` | 429 | Already have 10 pending events |
| `INVALID_EVENT_REVIEW` | 400 | Invalid review action |
| `VALIDATION_ERROR` | 400 | Invalid request body |
| `INTERNAL_ERROR` | 500 | Server error |
