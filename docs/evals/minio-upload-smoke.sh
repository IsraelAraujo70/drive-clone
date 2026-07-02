#!/usr/bin/env bash
set -euo pipefail

API_BASE_URL="${API_BASE_URL:-http://localhost:8080}"
TMP_FILE="$(mktemp)"
DOWNLOADED=""
EMAIL="smoke-$(date +%s)@example.com"
PASSWORD="password123"

cleanup() {
  rm -f "$TMP_FILE"
  if [[ -n "$DOWNLOADED" ]]; then
    rm -f "$DOWNLOADED"
  fi
}

trap cleanup EXIT

printf 'drive clone smoke\n' > "$TMP_FILE"
SIZE_BYTES="$(wc -c < "$TMP_FILE" | tr -d ' ')"
CHECKSUM_SHA256="$(shasum -a 256 "$TMP_FILE" | awk '{print $1}')"

SIGNUP_RESPONSE="$(
  curl -fsS -X POST "$API_BASE_URL/auth/signup" \
    -H 'content-type: application/json' \
    -d "{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\",\"display_name\":\"Smoke Test\"}"
)"
TOKEN="$(printf '%s' "$SIGNUP_RESPONSE" | python3 -c 'import json,sys; print(json.load(sys.stdin)["token"])')"

UPLOAD_RESPONSE="$(
  curl -fsS -X POST "$API_BASE_URL/files/uploads" \
    -H "authorization: Bearer $TOKEN" \
    -H 'content-type: application/json' \
    -d "{\"filename\":\"smoke.txt\",\"content_type\":\"text/plain\",\"size_bytes\":$SIZE_BYTES,\"checksum_sha256\":\"$CHECKSUM_SHA256\"}"
)"
FILE_ID="$(printf '%s' "$UPLOAD_RESPONSE" | python3 -c 'import json,sys; print(json.load(sys.stdin)["file_id"])')"
UPLOAD_URL="$(printf '%s' "$UPLOAD_RESPONSE" | python3 -c 'import json,sys; print(json.load(sys.stdin)["upload_url"])')"

curl -fsS -X PUT "$UPLOAD_URL" -H 'content-type: text/plain' --data-binary "@$TMP_FILE" >/dev/null

curl -fsS -X POST "$API_BASE_URL/files/$FILE_ID/complete" \
  -H "authorization: Bearer $TOKEN" >/dev/null

curl -fsS "$API_BASE_URL/files" -H "authorization: Bearer $TOKEN" \
  | python3 -c 'import json,sys; data=json.load(sys.stdin); assert len(data["files"]) == 1; assert data["files"][0]["state"] == "complete"'

DOWNLOAD_RESPONSE="$(curl -fsS "$API_BASE_URL/files/$FILE_ID/download" -H "authorization: Bearer $TOKEN")"
DOWNLOAD_URL="$(printf '%s' "$DOWNLOAD_RESPONSE" | python3 -c 'import json,sys; print(json.load(sys.stdin)["download_url"])')"
DOWNLOADED="$(mktemp)"
curl -fsS "$DOWNLOAD_URL" -o "$DOWNLOADED"

cmp "$TMP_FILE" "$DOWNLOADED"
echo "minio upload smoke passed: $FILE_ID"
