#!/usr/bin/env bash
set -euo pipefail

API_BASE_URL="${API_BASE_URL:-http://localhost:8080}"
WORK_DIR="$(mktemp -d)"
TMP_FILE="$WORK_DIR/resumable.bin"
PART_FILE="$WORK_DIR/part.bin"
HEADERS_FILE="$WORK_DIR/headers.txt"
DOWNLOADED="$WORK_DIR/downloaded.bin"
STAMP="$(date +%s)"
EMAIL="resumable-$STAMP@example.com"
PASSWORD="password123"
PART_SIZE_BYTES=$((6 * 1024 * 1024))
TOTAL_SIZE_BYTES=$((PART_SIZE_BYTES + 37))

cleanup() {
  rm -rf "$WORK_DIR"
}
trap cleanup EXIT

json_field() {
  python3 -c "import json,sys; print(json.load(sys.stdin)$1)"
}

python3 - "$TMP_FILE" "$TOTAL_SIZE_BYTES" <<'PY'
import sys
path = sys.argv[1]
size = int(sys.argv[2])
with open(path, "wb") as f:
    for i in range(size):
        f.write(bytes([i % 251]))
PY

CHECKSUM_SHA256="$(shasum -a 256 "$TMP_FILE" | awk '{print $1}')"

SIGNUP_RESPONSE="$(
  curl -fsS -X POST "$API_BASE_URL/auth/signup" \
    -H 'content-type: application/json' \
    -d "{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\",\"display_name\":\"Resumable Smoke\"}"
)"
TOKEN="$(printf '%s' "$SIGNUP_RESPONSE" | json_field '["token"]')"

CREATE_RESPONSE="$(
  curl -fsS -X POST "$API_BASE_URL/files/uploads/resumable" \
    -H "authorization: Bearer $TOKEN" \
    -H 'content-type: application/json' \
    -d "{\"filename\":\"resumable.bin\",\"content_type\":\"application/octet-stream\",\"size_bytes\":$TOTAL_SIZE_BYTES,\"checksum_sha256\":\"$CHECKSUM_SHA256\",\"part_size_bytes\":$PART_SIZE_BYTES}"
)"
FILE_ID="$(printf '%s' "$CREATE_RESPONSE" | json_field '["file_id"]')"

upload_part() {
  local part_number="$1" offset="$2" size_bytes="$3" sign_response upload_url etag
  sign_response="$(
    curl -fsS -X POST "$API_BASE_URL/files/uploads/$FILE_ID/parts" \
      -H "authorization: Bearer $TOKEN" \
      -H 'content-type: application/json' \
      -d "{\"part_number\":$part_number}"
  )"
  printf '%s' "$sign_response" | python3 -c "import json,sys; d=json.load(sys.stdin); assert d['part_number']==$part_number, d; assert d['expected_size_bytes']==$size_bytes, d"
  upload_url="$(printf '%s' "$sign_response" | json_field '["upload_url"]')"

  dd if="$TMP_FILE" of="$PART_FILE" bs=1 skip="$offset" count="$size_bytes" status=none
  curl -fsS -D "$HEADERS_FILE" -X PUT "$upload_url" --data-binary "@$PART_FILE" >/dev/null
  etag="$(python3 - "$HEADERS_FILE" <<'PY'
import sys
for line in open(sys.argv[1], encoding="utf-8", errors="ignore"):
    if line.lower().startswith("etag:"):
        print(line.split(":", 1)[1].strip())
        break
else:
    raise SystemExit("missing ETag")
PY
)"
  local record_body
  record_body="$(python3 - "$size_bytes" "$etag" <<'PY'
import json, sys
print(json.dumps({"size_bytes": int(sys.argv[1]), "etag": sys.argv[2]}))
PY
)"
  curl -fsS -X POST "$API_BASE_URL/files/uploads/$FILE_ID/parts/$part_number" \
    -H "authorization: Bearer $TOKEN" \
    -H 'content-type: application/json' \
    -d "$record_body" >/dev/null
}

upload_part 1 0 "$PART_SIZE_BYTES"

curl -fsS "$API_BASE_URL/files/uploads/$FILE_ID/status" \
  -H "authorization: Bearer $TOKEN" \
  | python3 -c "import json,sys; d=json.load(sys.stdin); assert d['state']=='pending', d; assert [p['part_number'] for p in d['parts']]==[1], d"

upload_part 2 "$PART_SIZE_BYTES" 37

curl -fsS -X POST "$API_BASE_URL/files/uploads/$FILE_ID/finalize" \
  -H "authorization: Bearer $TOKEN" >/dev/null

DOWNLOAD_RESPONSE="$(curl -fsS "$API_BASE_URL/files/$FILE_ID/download" -H "authorization: Bearer $TOKEN")"
DOWNLOAD_URL="$(printf '%s' "$DOWNLOAD_RESPONSE" | json_field '["download_url"]')"
curl -fsS "$DOWNLOAD_URL" -o "$DOWNLOADED"
cmp "$TMP_FILE" "$DOWNLOADED"

echo "resumable upload smoke passed against $API_BASE_URL (file $FILE_ID)"
