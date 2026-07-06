#!/usr/bin/env bash
set -euo pipefail

# E2E smoke for the per-owner cursor-based sync change feed against a live API.
# Usage: API_BASE_URL=https://api-production-bcad4.up.railway.app ./sync-changes-smoke.sh

API_BASE_URL="${API_BASE_URL:-http://localhost:8080}"
TMP_FILE="$(mktemp)"
STAMP="$(date +%s)"
EMAIL_OWNER="sync-owner-$STAMP@example.com"
EMAIL_OTHER="sync-other-$STAMP@example.com"
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

printf 'drive clone sync-changes smoke\n'

TOKEN_OWNER="$(signup "$EMAIL_OWNER" "Sync Owner")"
TOKEN_OTHER="$(signup "$EMAIL_OTHER" "Sync Other")"

FILE_ID="$(upload_file "$TOKEN_OWNER" "sync-report.txt" "sync report")"

# Rename (upsert) then trash (delete).
curl -fsS -X PATCH "$API_BASE_URL/files/$FILE_ID" \
  -H "authorization: Bearer $TOKEN_OWNER" \
  -H 'content-type: application/json' \
  -d '{"filename":"sync-renamed.txt"}' >/dev/null

curl -fsS -X DELETE "$API_BASE_URL/files/$FILE_ID" \
  -H "authorization: Bearer $TOKEN_OWNER" >/dev/null

# Full feed from the beginning: upsert (complete), upsert (rename), delete (trash).
curl -fsS "$API_BASE_URL/sync/changes?cursor=0" -H "authorization: Bearer $TOKEN_OWNER" \
  | python3 -c "
import json,sys
d=json.load(sys.stdin)
c=d['changes']
assert len(c)==3, d
ops=[e['op'] for e in c]
assert ops==['upsert','upsert','delete'], d
seqs=[e['seq'] for e in c]
assert seqs==[1,2,3], d
assert all(e['entity_id']=='$FILE_ID' for e in c), d
assert c[1]['file']['filename']=='sync-renamed.txt', d
assert 'file' not in c[2], d
assert d['next_cursor']==3, d
assert d['has_more'] is False, d
"

# Pagination: limit=1 drains one entry at a time and reports has_more.
curl -fsS "$API_BASE_URL/sync/changes?cursor=0&limit=1" -H "authorization: Bearer $TOKEN_OWNER" \
  | python3 -c "
import json,sys
d=json.load(sys.stdin)
assert len(d['changes'])==1, d
assert d['changes'][0]['seq']==1, d
assert d['next_cursor']==1, d
assert d['has_more'] is True, d
"

# Cursor past the end: empty, cursor unchanged.
curl -fsS "$API_BASE_URL/sync/changes?cursor=3" -H "authorization: Bearer $TOKEN_OWNER" \
  | python3 -c "
import json,sys
d=json.load(sys.stdin)
assert d['changes']==[], d
assert d['next_cursor']==3, d
assert d['has_more'] is False, d
"

# Isolation: the other user's feed never sees the owner's changes.
curl -fsS "$API_BASE_URL/sync/changes?cursor=0" -H "authorization: Bearer $TOKEN_OTHER" \
  | python3 -c "
import json,sys
d=json.load(sys.stdin)
assert d['changes']==[], d
assert d['next_cursor']==0, d
"

# Negative cursor is a client error.
STATUS="$(curl -s -o /dev/null -w '%{http_code}' \
  "$API_BASE_URL/sync/changes?cursor=-1" -H "authorization: Bearer $TOKEN_OWNER")"
test "$STATUS" = "422" || { echo "expected 422 for negative cursor, got $STATUS" >&2; exit 1; }

echo "sync-changes smoke passed against $API_BASE_URL (file $FILE_ID)"
