#!/usr/bin/env bash
set -euo pipefail

# E2E smoke for folder organization against a live API (local or prod).
# Usage: API_BASE_URL=https://api-production-bcad4.up.railway.app ./folder-organization-smoke.sh

API_BASE_URL="${API_BASE_URL:-http://localhost:8080}"
TMP_FILE="$(mktemp)"
DOWNLOADED=""
STAMP="$(date +%s)"
EMAIL_A="smoke-folder-owner-$STAMP@example.com"
EMAIL_B="smoke-folder-grantee-$STAMP@example.com"
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

signup() {
  curl -fsS -X POST "$API_BASE_URL/auth/signup" \
    -H 'content-type: application/json' \
    -d "{\"email\":\"$1\",\"password\":\"$PASSWORD\",\"display_name\":\"$2\"}" \
    | json_field '["token"]'
}

create_folder() {
  local token="$1" name="$2" parent="${3:-null}"
  curl -fsS -X POST "$API_BASE_URL/folders" \
    -H "authorization: Bearer $token" \
    -H 'content-type: application/json' \
    -d "{\"name\":\"$name\",\"parent_folder_id\":$parent}" \
    | json_field '["id"]'
}

printf 'drive clone folder smoke\n' > "$TMP_FILE"
SIZE_BYTES="$(wc -c < "$TMP_FILE" | tr -d ' ')"
CHECKSUM_SHA256="$(shasum -a 256 "$TMP_FILE" | awk '{print $1}')"

TOKEN_A="$(signup "$EMAIL_A" "Folder Owner")"
TOKEN_B="$(signup "$EMAIL_B" "Folder Grantee")"

PROJECTS_ID="$(create_folder "$TOKEN_A" "Projects")"
CLIENT_ID="$(create_folder "$TOKEN_A" "Client A" "\"$PROJECTS_ID\"")"

curl -fsS "$API_BASE_URL/drive" -H "authorization: Bearer $TOKEN_A" \
  | python3 -c "import json,sys; d=json.load(sys.stdin); assert any(f['id']=='$PROJECTS_ID' for f in d['folders']), d; assert d['files']==[], d"

UPLOAD_RESPONSE="$(
  curl -fsS -X POST "$API_BASE_URL/files/uploads" \
    -H "authorization: Bearer $TOKEN_A" \
    -H 'content-type: application/json' \
    -d "{\"filename\":\"brief.txt\",\"parent_folder_id\":\"$PROJECTS_ID\",\"content_type\":\"text/plain\",\"size_bytes\":$SIZE_BYTES,\"checksum_sha256\":\"$CHECKSUM_SHA256\"}"
)"
FILE_ID="$(printf '%s' "$UPLOAD_RESPONSE" | json_field '["file_id"]')"
UPLOAD_URL="$(printf '%s' "$UPLOAD_RESPONSE" | json_field '["upload_url"]')"
curl -fsS -X PUT "$UPLOAD_URL" -H 'content-type: text/plain' --data-binary "@$TMP_FILE" >/dev/null
curl -fsS -X POST "$API_BASE_URL/files/$FILE_ID/complete" -H "authorization: Bearer $TOKEN_A" >/dev/null

curl -fsS "$API_BASE_URL/drive?parent_folder_id=$PROJECTS_ID" -H "authorization: Bearer $TOKEN_A" \
  | python3 -c "import json,sys; d=json.load(sys.stdin); assert any(f['id']=='$CLIENT_ID' for f in d['folders']), d; assert any(f['id']=='$FILE_ID' for f in d['files']), d"

curl -fsS -X PATCH "$API_BASE_URL/files/$FILE_ID" \
  -H "authorization: Bearer $TOKEN_A" \
  -H 'content-type: application/json' \
  -d '{"filename":"brief-renamed.txt"}' >/dev/null

curl -fsS -X PATCH "$API_BASE_URL/folders/$PROJECTS_ID" \
  -H "authorization: Bearer $TOKEN_A" \
  -H 'content-type: application/json' \
  -d '{"name":"Work"}' >/dev/null

expect_status 404 PATCH "/files/$FILE_ID" "$TOKEN_B" '{"filename":"stolen.txt"}'
expect_status 409 PATCH "/folders/$PROJECTS_ID" "$TOKEN_A" "{\"parent_folder_id\":\"$CLIENT_ID\"}"

curl -fsS -X POST "$API_BASE_URL/files/$FILE_ID/shares" \
  -H "authorization: Bearer $TOKEN_A" \
  -H 'content-type: application/json' \
  -d "{\"email\":\"$EMAIL_B\"}" >/dev/null

expect_status 204 DELETE "/folders/$PROJECTS_ID" "$TOKEN_A"
expect_status 404 GET "/files/$FILE_ID/download" "$TOKEN_B"
curl -fsS "$API_BASE_URL/files/shared-with-me" -H "authorization: Bearer $TOKEN_B" \
  | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["files"]==[], d'
curl -fsS "$API_BASE_URL/drive/trash" -H "authorization: Bearer $TOKEN_A" \
  | python3 -c "import json,sys; d=json.load(sys.stdin); assert any(f['id']=='$PROJECTS_ID' for f in d['folders']), d; assert d['files']==[], d"

expect_status 200 POST "/folders/$PROJECTS_ID/restore" "$TOKEN_A"
DOWNLOAD_URL="$(curl -fsS "$API_BASE_URL/files/$FILE_ID/download" -H "authorization: Bearer $TOKEN_B" | json_field '["download_url"]')"
DOWNLOADED="$(mktemp)"
curl -fsS "$DOWNLOAD_URL" -o "$DOWNLOADED"
cmp "$TMP_FILE" "$DOWNLOADED"

curl -fsS -X PATCH "$API_BASE_URL/files/$FILE_ID" \
  -H "authorization: Bearer $TOKEN_A" \
  -H 'content-type: application/json' \
  -d '{"parent_folder_id":null}' >/dev/null
curl -fsS "$API_BASE_URL/drive" -H "authorization: Bearer $TOKEN_A" \
  | python3 -c "import json,sys; d=json.load(sys.stdin); assert any(f['id']=='$FILE_ID' and f['parent_folder_id'] is None for f in d['files']), d"

echo "folder organization smoke passed against $API_BASE_URL (file $FILE_ID, folder $PROJECTS_ID)"
