#!/usr/bin/env bash
set -euo pipefail

API_BASE_URL="${API_BASE_URL:-https://api.drive.israeldeveloper.com.br}"
WEB_ORIGIN="${WEB_ORIGIN:-https://drive.israeldeveloper.com.br}"
SIZE_BYTES="${SIZE_BYTES:-1350000}"
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/drive-upload-cors.XXXXXX")"

cleanup() {
  rm -rf "$WORK_DIR"
}
trap cleanup EXIT

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

extract_header() {
  local header_name="$1"
  local file="$2"
  awk -v wanted="$(printf '%s' "$header_name" | tr '[:upper:]' '[:lower:]')" '
    {
      line = $0
      sub(/\r$/, "", line)
      split(line, parts, ":")
      name = tolower(parts[1])
      if (name == wanted) {
        sub(/^[^:]*:[[:space:]]*/, "", line)
        value = line
      }
    }
    END {
      if (value != "") {
        print value
      }
    }
  ' "$file"
}

require_command curl
require_command jq
require_command cmp
require_command head

upload_bytes="$WORK_DIR/upload.bin"
download_bytes="$WORK_DIR/download.bin"
head -c "$SIZE_BYTES" /dev/urandom > "$upload_bytes"

email="cors-smoke-$(date +%s)-$RANDOM@example.com"
password="Password123!"

auth_response="$(
  jq -n \
    --arg email "$email" \
    --arg password "$password" \
    --arg display_name "CORS Smoke" \
    '{email:$email,password:$password,display_name:$display_name}' |
    curl -fsS "$API_BASE_URL/auth/signup" \
      -H "content-type: application/json" \
      --data @-
)"
token="$(printf '%s' "$auth_response" | jq -r .token)"

session_response="$(
  jq -n \
    --arg filename "cors-smoke.bin" \
    --arg content_type "application/octet-stream" \
    --argjson size_bytes "$SIZE_BYTES" \
    '{
      filename:$filename,
      parent_folder_id:null,
      content_type:$content_type,
      size_bytes:$size_bytes,
      checksum_sha256:null
    }' |
    curl -fsS "$API_BASE_URL/files/uploads/resumable" \
      -H "authorization: Bearer $token" \
      -H "content-type: application/json" \
      --data @-
)"
file_id="$(printf '%s' "$session_response" | jq -r .file_id)"

part_response="$(
  curl -fsS "$API_BASE_URL/files/uploads/$file_id/parts" \
    -H "authorization: Bearer $token" \
    -H "content-type: application/json" \
    --data '{"part_number":1}'
)"
upload_url="$(printf '%s' "$part_response" | jq -r .upload_url)"

preflight_headers="$WORK_DIR/preflight.headers"
curl -fsS -D "$preflight_headers" -o /dev/null \
  -X OPTIONS "$upload_url" \
  -H "Origin: $WEB_ORIGIN" \
  -H "Access-Control-Request-Method: PUT"

allowed_origin="$(extract_header "access-control-allow-origin" "$preflight_headers")"
if [[ "$allowed_origin" != "$WEB_ORIGIN" ]]; then
  echo "bucket CORS did not allow $WEB_ORIGIN; got: ${allowed_origin:-missing}" >&2
  exit 1
fi

put_headers="$WORK_DIR/put.headers"
curl -fsS -D "$put_headers" -o /dev/null \
  -X PUT "$upload_url" \
  -H "Origin: $WEB_ORIGIN" \
  --data-binary @"$upload_bytes"

etag="$(extract_header "etag" "$put_headers")"
if [[ -z "$etag" ]]; then
  echo "part upload succeeded without an ETag header" >&2
  exit 1
fi

jq -n \
  --arg etag "$etag" \
  --argjson size_bytes "$SIZE_BYTES" \
  '{size_bytes:$size_bytes,etag:$etag}' |
  curl -fsS "$API_BASE_URL/files/uploads/$file_id/parts/1" \
    -H "authorization: Bearer $token" \
    -H "content-type: application/json" \
    --data @- >/dev/null

curl -fsS -X POST "$API_BASE_URL/files/uploads/$file_id/finalize" \
  -H "authorization: Bearer $token" >/dev/null

download_url="$(
  curl -fsS "$API_BASE_URL/files/$file_id/download" \
    -H "authorization: Bearer $token" |
    jq -r .download_url
)"
curl -fsS "$download_url" -o "$download_bytes"
cmp -s "$upload_bytes" "$download_bytes"

printf 'production upload CORS smoke passed for %s using file_id=%s\n' \
  "$WEB_ORIGIN" "$file_id"
