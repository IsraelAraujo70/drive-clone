#!/usr/bin/env bash
set -euo pipefail

# E2E smoke for sharing + soft delete against a live API (local or prod).
# Usage: API_BASE_URL=https://api-production-bcad4.up.railway.app ./share-delete-smoke.sh

API_BASE_URL="${API_BASE_URL:-http://localhost:8080}"
TMP_FILE="$(mktemp)"
DOWNLOADED=""
STAMP="$(date +%s)"
EMAIL_A="smoke-owner-$STAMP@example.com"
EMAIL_B="smoke-grantee-$STAMP@example.com"
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

# expect_status <expected> <method> <path> <token> [json body]
expect_status() {
  local expected="$1" method="$2" path="$3" token="$4" body="${5:-}"
  local args=(-s -o /dev/null -w '%{http_code}' -X "$method" "$API_BASE_URL$path" -H "authorization: Bearer $token")
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

printf 'drive clone share smoke\n' > "$TMP_FILE"
SIZE_BYTES="$(wc -c < "$TMP_FILE" | tr -d ' ')"
CHECKSUM_SHA256="$(shasum -a 256 "$TMP_FILE" | awk '{print $1}')"

signup() {
  curl -fsS -X POST "$API_BASE_URL/auth/signup" \
    -H 'content-type: application/json' \
    -d "{\"email\":\"$1\",\"password\":\"$PASSWORD\",\"display_name\":\"$2\"}" \
    | json_field '["token"]'
}

TOKEN_A="$(signup "$EMAIL_A" "Smoke Owner")"
TOKEN_B="$(signup "$EMAIL_B" "Smoke Grantee")"

# Owner uploads and completes a file.
UPLOAD_RESPONSE="$(
  curl -fsS -X POST "$API_BASE_URL/files/uploads" \
    -H "authorization: Bearer $TOKEN_A" \
    -H 'content-type: application/json' \
    -d "{\"filename\":\"smoke-share.txt\",\"content_type\":\"text/plain\",\"size_bytes\":$SIZE_BYTES,\"checksum_sha256\":\"$CHECKSUM_SHA256\"}"
)"
FILE_ID="$(printf '%s' "$UPLOAD_RESPONSE" | json_field '["file_id"]')"
UPLOAD_URL="$(printf '%s' "$UPLOAD_RESPONSE" | json_field '["upload_url"]')"
curl -fsS -X PUT "$UPLOAD_URL" -H 'content-type: text/plain' --data-binary "@$TMP_FILE" >/dev/null
curl -fsS -X POST "$API_BASE_URL/files/$FILE_ID/complete" -H "authorization: Bearer $TOKEN_A" >/dev/null

# Private by default: grantee cannot download before the share exists.
expect_status 404 GET "/files/$FILE_ID/download" "$TOKEN_B"

# Share validations: unknown email 404, self-share 422, real share 201, idempotent re-share 201.
expect_status 404 POST "/files/$FILE_ID/shares" "$TOKEN_A" "{\"email\":\"nobody-$STAMP@example.com\"}"
expect_status 422 POST "/files/$FILE_ID/shares" "$TOKEN_A" "{\"email\":\"$EMAIL_A\"}"
expect_status 201 POST "/files/$FILE_ID/shares" "$TOKEN_A" "{\"email\":\"$EMAIL_B\"}"
expect_status 201 POST "/files/$FILE_ID/shares" "$TOKEN_A" "{\"email\":\"$EMAIL_B\"}"

# Owner sees exactly one grantee; grantee id captured for revoke later.
GRANTEE_ID="$(curl -fsS "$API_BASE_URL/files/$FILE_ID/shares" -H "authorization: Bearer $TOKEN_A" \
  | python3 -c "import json,sys; d=json.load(sys.stdin); assert len(d['shares'])==1, d; s=d['shares'][0]; assert s['grantee']['email']=='$EMAIL_B', s; print(s['grantee']['id'])")"

# Grantee sees the file in shared-with-me with the owner attached.
curl -fsS "$API_BASE_URL/files/shared-with-me" -H "authorization: Bearer $TOKEN_B" \
  | python3 -c "import json,sys; d=json.load(sys.stdin); assert len(d['files'])==1, d; f=d['files'][0]; assert f['id']=='$FILE_ID', f; assert f['owner']['email']=='$EMAIL_A', f"

# Grantee downloads and the bytes match.
DOWNLOAD_URL="$(curl -fsS "$API_BASE_URL/files/$FILE_ID/download" -H "authorization: Bearer $TOKEN_B" | json_field '["download_url"]')"
DOWNLOADED="$(mktemp)"
curl -fsS "$DOWNLOAD_URL" -o "$DOWNLOADED"
cmp "$TMP_FILE" "$DOWNLOADED"

# Soft delete: gone from the owner's list, share stops granting access, shows up in trash.
expect_status 204 DELETE "/files/$FILE_ID" "$TOKEN_A"
curl -fsS "$API_BASE_URL/files" -H "authorization: Bearer $TOKEN_A" \
  | python3 -c 'import json,sys; d=json.load(sys.stdin); assert len(d["files"])==0, d'
expect_status 404 GET "/files/$FILE_ID/download" "$TOKEN_B"
curl -fsS "$API_BASE_URL/files/shared-with-me" -H "authorization: Bearer $TOKEN_B" \
  | python3 -c 'import json,sys; d=json.load(sys.stdin); assert len(d["files"])==0, d'
curl -fsS "$API_BASE_URL/files/trash" -H "authorization: Bearer $TOKEN_A" \
  | python3 -c "import json,sys; d=json.load(sys.stdin); assert len(d['files'])==1, d; assert d['files'][0]['deleted_at'], d"

# Restore: back in the list, grantee can download again.
expect_status 200 POST "/files/$FILE_ID/restore" "$TOKEN_A"
curl -fsS "$API_BASE_URL/files" -H "authorization: Bearer $TOKEN_A" \
  | python3 -c 'import json,sys; d=json.load(sys.stdin); assert len(d["files"])==1, d'
expect_status 200 GET "/files/$FILE_ID/download" "$TOKEN_B"

# Revoke: grantee loses access for good.
expect_status 204 DELETE "/files/$FILE_ID/shares/$GRANTEE_ID" "$TOKEN_A"
expect_status 404 GET "/files/$FILE_ID/download" "$TOKEN_B"
curl -fsS "$API_BASE_URL/files/shared-with-me" -H "authorization: Bearer $TOKEN_B" \
  | python3 -c 'import json,sys; d=json.load(sys.stdin); assert len(d["files"])==0, d'

echo "share/delete smoke passed against $API_BASE_URL (file $FILE_ID)"
