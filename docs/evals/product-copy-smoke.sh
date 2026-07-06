#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LANDING="$ROOT_DIR/apps/web/components/landing.tsx"
LAYOUT="$ROOT_DIR/apps/web/app/layout.tsx"

PUBLIC_COPY="$LANDING $LAYOUT"

fail() {
  echo "product-copy smoke failed: $*" >&2
  exit 1
}

for forbidden in "resumable" "resume" "chunk" "share link"; do
  if grep -Eiq "$forbidden" $PUBLIC_COPY; then
    fail "public landing/metadata still contains forbidden copy: $forbidden"
  fi
done

grep -Eiq "signed direct uploads|signs direct uploads" "$LANDING" \
  || fail "landing does not mention signed/direct upload"

grep -Eiq "email|registered account|account-based sharing" "$LANDING" \
  || fail "landing does not mention email/account sharing"

grep -Eiq "search" "$LANDING" \
  || fail "landing does not mention search"

grep -Eiq "folders|folder|trash|restore" "$LANDING" \
  || fail "landing does not mention folders/trash/restore"

echo "product-copy smoke passed"
