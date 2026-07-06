#!/usr/bin/env bash
set -euo pipefail

# E2E smoke for ACL-scoped filename search against a live API.
# Usage: API_BASE_URL=https://api-production-bcad4.up.railway.app ./search-smoke.sh

API_BASE_URL="${API_BASE_URL:-http://localhost:8080}"
TMP_FILE="$(mktemp)"
STAMP="$(date +%s)"
EMAIL_OWNER="search-owner-$STAMP@example.com"
EMAIL_GRANTEE="search-grantee-$STAMP@example.com"
EMAIL_OTHER="search-other-$STAMP@example.com"
PASSWORD="password123"

cleanup() {
  rm -f "$TMP_FILE"
}
trap cleanup EXIT

json_field() {
  python3 -c "import json,sys; print(json.load(sys.stdin)$1)"
}

signup() {
  curl -fsS -X POST "$API_BASE_URL/auth/signup" \
    -H 'content-type: application/json' \
    -d "{\"email\":\"$1\",\"password\":\"$PASSWORD\",\"display_name\":\"$2\"}" \
    | json_field '["token"]'
}

upload_file() {
  local token="$1" filename="$2" content="$3"
  printf '%s\n' "$content" > "$TMP_FILE"
  local size_bytes checksum upload_response file_id upload_url
  size_bytes="$(wc -c < "$TMP_FILE" | tr -d ' ')"
  checksum="$(shasum -a 256 "$TMP_FILE" | awk '{print $1}')"
  upload_response="$(
    curl -fsS -X POST "$API_BASE_URL/files/uploads" \
      -H "authorization: Bearer $token" \
      -H 'content-type: application/json' \
      -d "{\"filename\":\"$filename\",\"content_type\":\"text/plain\",\"size_bytes\":$size_bytes,\"checksum_sha256\":\"$checksum\"}"
  )"
  file_id="$(printf '%s' "$upload_response" | json_field '["file_id"]')"
  upload_url="$(printf '%s' "$upload_response" | json_field '["upload_url"]')"
  curl -fsS -X PUT "$upload_url" -H 'content-type: text/plain' --data-binary "@$TMP_FILE" >/dev/null
  curl -fsS -X POST "$API_BASE_URL/files/$file_id/complete" -H "authorization: Bearer $token" >/dev/null
  printf '%s' "$file_id"
}

printf 'drive clone search smoke\n'

TOKEN_OWNER="$(signup "$EMAIL_OWNER" "Search Owner")"
TOKEN_GRANTEE="$(signup "$EMAIL_GRANTEE" "Search Grantee")"
TOKEN_OTHER="$(signup "$EMAIL_OTHER" "Search Other")"

QUARTERLY_ID="$(upload_file "$TOKEN_OWNER" "quarterly-report.txt" "quarterly report")"
SHARED_ID="$(upload_file "$TOKEN_OWNER" "shared-report.txt" "shared report")"
DELETED_ID="$(upload_file "$TOKEN_OWNER" "deleted-report.txt" "deleted report")"
PRIVATE_ID="$(upload_file "$TOKEN_OTHER" "private-report.txt" "private report")"

curl -fsS -X POST "$API_BASE_URL/files/$SHARED_ID/shares" \
  -H "authorization: Bearer $TOKEN_OWNER" \
  -H 'content-type: application/json' \
  -d "{\"email\":\"$EMAIL_GRANTEE\"}" >/dev/null

curl -fsS -X DELETE "$API_BASE_URL/files/$DELETED_ID" \
  -H "authorization: Bearer $TOKEN_OWNER" >/dev/null

curl -fsS "$API_BASE_URL/search?q=report" -H "authorization: Bearer $TOKEN_GRANTEE" \
  | python3 -c "import json,sys; d=json.load(sys.stdin); ids=[r['file']['id'] for r in d['files']]; assert '$SHARED_ID' in ids, d; assert '$QUARTERLY_ID' not in ids, d; assert '$PRIVATE_ID' not in ids, d; assert '$DELETED_ID' not in ids, d; shared=[r for r in d['files'] if r['file']['id']=='$SHARED_ID'][0]; assert shared['access']=='shared', shared; assert shared['owner']['email']=='$EMAIL_OWNER', shared"

curl -fsS "$API_BASE_URL/search?q=report" -H "authorization: Bearer $TOKEN_OWNER" \
  | python3 -c "import json,sys; d=json.load(sys.stdin); ids=[r['file']['id'] for r in d['files']]; assert '$QUARTERLY_ID' in ids, d; assert '$SHARED_ID' in ids, d; assert '$PRIVATE_ID' not in ids, d; assert '$DELETED_ID' not in ids, d"

curl -fsS "$API_BASE_URL/search?q=deleted&include_deleted=true" -H "authorization: Bearer $TOKEN_OWNER" \
  | python3 -c "import json,sys; d=json.load(sys.stdin); ids=[r['file']['id'] for r in d['files']]; assert ids==['$DELETED_ID'], d; assert d['files'][0]['access']=='owned', d"

echo "search smoke passed against $API_BASE_URL (shared $SHARED_ID, deleted $DELETED_ID)"
