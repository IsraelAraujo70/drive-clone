#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
API_DIR="$ROOT_DIR/services/api"

export DATABASE_URL="${DATABASE_URL:-postgres://postgres:postgres@localhost:5433/drive_clone}"

cd "$API_DIR"

cargo test purge_trash_deletes_bucket_row_and_quota_but_keeps_sync_tombstone --test api_contract
cargo test purge_claim_blocks_duplicate_workers_and_restore_until_released --test api_contract
cargo test stale_purge_claim_cannot_release_or_purge_new_claim --test api_contract
cargo test reconcile_quota_corrects_corrupted_storage_used_bytes --test api_contract
