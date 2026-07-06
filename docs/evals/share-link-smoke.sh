#!/usr/bin/env bash
set -euo pipefail

# E2E smoke for public, revocable share links against a live API (local or prod).
# Usage: API_BASE_URL=https://api-production-bcad4.up.railway.app ./share-link-smoke.sh

API_BASE_URL="${API_BASE_URL:-http://localhost:8080}"
TMP_FILE="$(mktemp)"
DOWNLOADED=""
STAMP="$(date +%s)"
EMAIL_A="link-owner-$STAMP@example.com"
EMAIL_B="link-other-$STAMP@example.com"
PASSWORD="password123"

cleanup() {
  rm -f "$TMP_FILE"
  if [[ -n "$DOWNLOADED" ]]; then
    rm -f "$DOWNLOADED"
  fi
}
trap cleanup EXIT

json_field() {
  python3 -c "import json,sys; print(json.load(sys.stdin)$1)"
}

# expect_status <expected> <method> <path> <token|-> [json body]
expect_status() {
  local expected="$1" method="$2" path="$3" token="$4" body="${5:-}"
  local args=(-s -o /dev/null -w '%{http_code}' -X "$method" "$API_BASE_URL$path")
  if [[ "$token" != "-" ]]; then
    args+=(-H "authorization: Bearer $token")
  fi
  if [[ -n "$body" ]]; then
    args+=(-H 'content-type: application/json' -d "$body")
  fi
  local status
  status="$(curl "${args[@]}")"
  if [[ "$status" != "$expected" ]]; then
    echo "FAIL: $method $path expected $expected got $status" >&2
    exit 1
  fi
}

printf 'drive clone share-link smoke\n' > "$TMP_FILE"
SIZE_BYTES="$(wc -c < "$TMP_FILE" | tr -d ' ')"
CHECKSUM_SHA256="$(shasum -a 256 "$TMP_FILE" | awk '{print $1}')"

signup() {
  curl -fsS -X POST "$API_BASE_URL/auth/signup" \
    -H 'content-type: application/json' \
    -d "{\"email\":\"$1\",\"password\":\"$PASSWORD\",\"display_name\":\"$2\"}" \
    | json_field '["token"]'
}

TOKEN_A="$(signup "$EMAIL_A" "Link Owner")"
TOKEN_B="$(signup "$EMAIL_B" "Link Other")"

# Owner uploads and completes a file.
UPLOAD_RESPONSE="$(
  curl -fsS -X POST "$API_BASE_URL/files/uploads" \
    -H "authorization: Bearer $TOKEN_A" \
    -H 'content-type: application/json' \
    -d "{\"filename\":\"smoke-link.txt\",\"content_type\":\"text/plain\",\"size_bytes\":$SIZE_BYTES,\"checksum_sha256\":\"$CHECKSUM_SHA256\"}"
)"
FILE_ID="$(printf '%s' "$UPLOAD_RESPONSE" | json_field '["file_id"]')"
UPLOAD_URL="$(printf '%s' "$UPLOAD_RESPONSE" | json_field '["upload_url"]')"
curl -fsS -X PUT "$UPLOAD_URL" -H 'content-type: text/plain' --data-binary "@$TMP_FILE" >/dev/null
curl -fsS -X POST "$API_BASE_URL/files/$FILE_ID/complete" -H "authorization: Bearer $TOKEN_A" >/dev/null

# Non-owner cannot create a link; non-positive expiry is rejected.
expect_status 404 POST "/files/$FILE_ID/share-links" "$TOKEN_B" '{"expires_in_seconds": null}'
expect_status 422 POST "/files/$FILE_ID/share-links" "$TOKEN_A" '{"expires_in_seconds": 0}'

# Owner creates a non-expiring link; token + url returned once.
CREATE_RESPONSE="$(curl -fsS -X POST "$API_BASE_URL/files/$FILE_ID/share-links" \
  -H "authorization: Bearer $TOKEN_A" -H 'content-type: application/json' \
  -d '{"expires_in_seconds": null}')"
LINK_ID="$(printf '%s' "$CREATE_RESPONSE" | json_field '["id"]')"
TOKEN="$(printf '%s' "$CREATE_RESPONSE" | json_field '["token"]')"
printf '%s' "$CREATE_RESPONSE" | python3 -c "import json,sys; d=json.load(sys.stdin); assert d['url'].endswith('/s/'+d['token']), d; assert d['expires_at'] is None, d"

# Public, unauthenticated resolve returns metadata and a working download URL.
PUBLIC="$(curl -fsS "$API_BASE_URL/shared/links/$TOKEN")"
printf '%s' "$PUBLIC" | python3 -c "import json,sys; d=json.load(sys.stdin); assert d['filename']=='smoke-link.txt', d; assert d['size_bytes']==$SIZE_BYTES, d; assert d['content_type']=='text/plain', d"
DOWNLOAD_URL="$(printf '%s' "$PUBLIC" | json_field '["download_url"]')"
DOWNLOADED="$(mktemp)"
curl -fsS "$DOWNLOAD_URL" -o "$DOWNLOADED"
cmp "$TMP_FILE" "$DOWNLOADED"

# Owner lists the link without exposing the token.
curl -fsS "$API_BASE_URL/files/$FILE_ID/share-links" -H "authorization: Bearer $TOKEN_A" \
  | python3 -c "import json,sys; d=json.load(sys.stdin); assert len(d['links'])==1, d; l=d['links'][0]; assert l['id']=='$LINK_ID', l; assert 'token' not in l, l"

# Non-owner cannot list.
expect_status 404 GET "/files/$FILE_ID/share-links" "$TOKEN_B"

# Wrong token → uniform 404.
expect_status 404 GET "/shared/links/not-a-real-token" -

# Revoke → resolve becomes 404 (unauthenticated).
expect_status 404 DELETE "/files/$FILE_ID/share-links/$LINK_ID" "$TOKEN_B"
expect_status 204 DELETE "/files/$FILE_ID/share-links/$LINK_ID" "$TOKEN_A"
expect_status 404 GET "/shared/links/$TOKEN" -

# Expiry: a 1-second link stops resolving after it lapses.
EXPIRING="$(curl -fsS -X POST "$API_BASE_URL/files/$FILE_ID/share-links" \
  -H "authorization: Bearer $TOKEN_A" -H 'content-type: application/json' \
  -d '{"expires_in_seconds": 1}')"
EXPIRING_TOKEN="$(printf '%s' "$EXPIRING" | json_field '["token"]')"
expect_status 200 GET "/shared/links/$EXPIRING_TOKEN" -
sleep 2
expect_status 404 GET "/shared/links/$EXPIRING_TOKEN" -

# Trashed file: a live link stops resolving once the file is trashed.
LIVE="$(curl -fsS -X POST "$API_BASE_URL/files/$FILE_ID/share-links" \
  -H "authorization: Bearer $TOKEN_A" -H 'content-type: application/json' \
  -d '{"expires_in_seconds": null}')"
LIVE_TOKEN="$(printf '%s' "$LIVE" | json_field '["token"]')"
expect_status 200 GET "/shared/links/$LIVE_TOKEN" -
expect_status 204 DELETE "/files/$FILE_ID" "$TOKEN_A"
expect_status 404 GET "/shared/links/$LIVE_TOKEN" -

echo "share-link smoke passed against $API_BASE_URL (file $FILE_ID)"
