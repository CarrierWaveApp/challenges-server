#!/usr/bin/env bash
# End-to-end tests for the challenges server API.
# Requires: BASE_URL and ADMIN_TOKEN environment variables.
# Exits non-zero on first failure.

set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
ADMIN_TOKEN="${ADMIN_TOKEN:-e2e-admin-token}"

PASS=0
FAIL=0

# ── Helpers ──────────────────────────────────────────────────────────────────

assert_status() {
  local description="$1"
  local expected="$2"
  local actual="$3"

  if [ "$actual" -eq "$expected" ]; then
    echo "  PASS: $description (HTTP $actual)"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $description — expected $expected, got $actual"
    FAIL=$((FAIL + 1))
  fi
}

get() {
  curl -s -o /dev/null -w "%{http_code}" "$BASE_URL$1"
}

get_json() {
  curl -sf "$BASE_URL$1"
}

post() {
  local path="$1"
  shift
  curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL$path" \
    -H "Content-Type: application/json" "$@"
}

post_json() {
  local path="$1"
  shift
  curl -sf -X POST "$BASE_URL$path" \
    -H "Content-Type: application/json" "$@"
}

delete() {
  curl -s -o /dev/null -w "%{http_code}" -X DELETE "$BASE_URL$1" \
    "${@:2}"
}

admin_header=(-H "Authorization: Bearer $ADMIN_TOKEN")

# ── Health ───────────────────────────────────────────────────────────────────

echo "=== Health ==="
assert_status "GET /v1/health" 200 "$(get /v1/health)"

health_body=$(get_json /v1/health)
if echo "$health_body" | grep -q '"status"'; then
  echo "  PASS: health response contains status field"
  PASS=$((PASS + 1))
else
  echo "  FAIL: health response missing status field"
  FAIL=$((FAIL + 1))
fi

# ── Public endpoints (empty state) ──────────────────────────────────────────

echo "=== Public endpoints ==="
assert_status "GET /v1/challenges" 200 "$(get /v1/challenges)"
assert_status "GET /v1/programs" 200 "$(get /v1/programs)"
assert_status "GET /v1/pota/stats/status" 200 "$(get /v1/pota/stats/status)"
assert_status "GET /v1/pota/stats/rankings/activators" 200 "$(get '/v1/pota/stats/rankings/activators?limit=10')"

# ── Admin auth ──────────────────────────────────────────────────────────────

echo "=== Admin auth ==="
assert_status "POST /v1/admin/challenges (no auth)" 401 \
  "$(post /v1/admin/challenges -d '{"name":"test"}')"

# ── Challenge CRUD flow ─────────────────────────────────────────────────────

echo "=== Challenge lifecycle ==="

# Create a challenge
challenge_response=$(post_json /v1/admin/challenges \
  "${admin_header[@]}" \
  -d '{
    "name": "E2E Test Challenge",
    "description": "Created by e2e tests",
    "challenge_type": "dxcc_entities",
    "target_count": 10,
    "start_date": "2025-01-01T00:00:00Z",
    "end_date": "2026-12-31T23:59:59Z"
  }')

challenge_id=$(echo "$challenge_response" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)

if [ -n "$challenge_id" ]; then
  echo "  PASS: Created challenge $challenge_id"
  PASS=$((PASS + 1))
else
  echo "  FAIL: Could not create challenge"
  FAIL=$((FAIL + 1))
  echo ""
  echo "=== Results: $PASS passed, $FAIL failed ==="
  exit 1
fi

# Get the challenge
assert_status "GET /v1/challenges/$challenge_id" 200 \
  "$(get "/v1/challenges/$challenge_id")"

# Get leaderboard
assert_status "GET /v1/challenges/$challenge_id/leaderboard" 200 \
  "$(get "/v1/challenges/$challenge_id/leaderboard")"

# Join the challenge (get a device token first)
join_response=$(post_json "/v1/challenges/$challenge_id/join" \
  -d '{"callsign": "E2ETEST"}')
device_token=$(echo "$join_response" | grep -o '"device_token":"[^"]*"' | head -1 | cut -d'"' -f4)

if [ -n "$device_token" ]; then
  echo "  PASS: Joined challenge, got device token"
  PASS=$((PASS + 1))
else
  echo "  FAIL: Could not join challenge"
  FAIL=$((FAIL + 1))
fi

# Report progress (auth required)
if [ -n "$device_token" ]; then
  auth_header=(-H "Authorization: Bearer $device_token")

  progress_status=$(post "/v1/challenges/$challenge_id/progress" \
    "${auth_header[@]}" \
    -d '{"entry": "K1ABC", "band": "20m", "mode": "SSB"}')
  assert_status "POST progress (authenticated)" 200 "$progress_status"

  # Get own progress
  assert_status "GET /v1/challenges/$challenge_id/progress" 200 \
    "$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/v1/challenges/$challenge_id/progress" "${auth_header[@]}")"

  # Leave challenge
  leave_status=$(delete "/v1/challenges/$challenge_id/leave" "${auth_header[@]}")
  assert_status "DELETE /v1/challenges/$challenge_id/leave" 200 "$leave_status"
fi

# Admin delete
admin_delete_status=$(delete "/v1/admin/challenges/$challenge_id" "${admin_header[@]}")
assert_status "DELETE /v1/admin/challenges/$challenge_id (admin)" 200 "$admin_delete_status"

# Verify deleted
assert_status "GET /v1/challenges/$challenge_id (after delete)" 404 \
  "$(get "/v1/challenges/$challenge_id")"

# ── Results ──────────────────────────────────────────────────────────────────

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
