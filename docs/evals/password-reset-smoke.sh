#!/usr/bin/env bash
set -euo pipefail

API_BASE_URL="${API_BASE_URL:-http://localhost:8080}"
STAMP="$(date +%s)"
EMAIL="reset-${STAMP}@example.com"
OLD_PASSWORD="password123"
NEW_PASSWORD="Password123!"
RAW_RESET_TOKEN="eval-reset-${STAMP}"

fail() {
  echo "password-reset-smoke: $*" >&2
  exit 1
}

for attempt in {1..60}; do
  if curl -fsS "$API_BASE_URL/health" >/dev/null 2>&1; then
    break
  fi
  if [[ "$attempt" -eq 60 ]]; then
    fail "API did not become healthy at $API_BASE_URL"
  fi
  sleep 1
done

api() {
  local method="$1"
  local path="$2"
  local body="${3:-}"
  local token="${4:-}"
  local args=(-fsS -X "$method" "$API_BASE_URL$path")
  if [[ -n "$token" ]]; then
    args+=(-H "Authorization: Bearer $token")
  fi
  if [[ -n "$body" ]]; then
    args+=(-H "Content-Type: application/json" -d "$body")
  fi
  curl "${args[@]}"
}

expect_status() {
  local expected="$1"
  local method="$2"
  local path="$3"
  local body="${4:-}"
  local token="${5:-}"
  local status
  status="$(
    curl -sS -o /tmp/password-reset-smoke-response.json -w "%{http_code}" \
      -X "$method" "$API_BASE_URL$path" \
      ${token:+-H "Authorization: Bearer $token"} \
      ${body:+-H "Content-Type: application/json" -d "$body"}
  )"
  [[ "$status" == "$expected" ]] || fail "expected $method $path to return $expected, got $status: $(cat /tmp/password-reset-smoke-response.json)"
}

TOKEN="$(
  api POST /auth/signup \
    "{\"email\":\"$EMAIL\",\"password\":\"$OLD_PASSWORD\",\"display_name\":\"Reset Smoke\"}" \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["token"])'
)"

expect_status 204 POST /auth/password/forgot "{\"email\":\"missing-${STAMP}@example.com\"}"
expect_status 204 POST /auth/password/forgot "{\"email\":\"$EMAIL\"}"

USER_ID="$(
  docker compose exec -T postgres psql -U postgres -d drive_clone -Atc \
    "SELECT id FROM users WHERE email = '$EMAIL'"
)"
[[ -n "$USER_ID" ]] || fail "created user was not found in postgres"

TOKEN_HASH="$(python3 -c 'import hashlib,sys; print(hashlib.sha256(sys.argv[1].encode()).hexdigest())' "$RAW_RESET_TOKEN")"
docker compose exec -T postgres psql -U postgres -d drive_clone \
  -c "INSERT INTO password_reset_tokens (user_id, token_hash, expires_at) VALUES ('$USER_ID'::uuid, '$TOKEN_HASH', now() + interval '1 hour')" \
  >/dev/null

expect_status 204 POST /auth/password/reset "{\"token\":\"$RAW_RESET_TOKEN\",\"password\":\"$NEW_PASSWORD\"}"
expect_status 401 GET /auth/me "" "$TOKEN"
expect_status 401 POST /auth/login "{\"email\":\"$EMAIL\",\"password\":\"$OLD_PASSWORD\"}"

api POST /auth/login "{\"email\":\"$EMAIL\",\"password\":\"$NEW_PASSWORD\"}" \
  | python3 -c 'import json,sys; assert len(json.load(sys.stdin)["token"]) > 40'

expect_status 422 POST /auth/password/reset "{\"token\":\"$RAW_RESET_TOKEN\",\"password\":\"Password123!!\"}"

echo "password-reset-smoke: ok"
