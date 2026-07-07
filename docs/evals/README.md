# Evals

## Product Copy Smoke

Runs locally without services:

```bash
bash docs/evals/product-copy-smoke.sh
```

The script checks only the public landing page and metadata. It fails if those
surfaces advertise upload recovery, object parts, or share links before those
features exist, and confirms the landing mentions the implemented direct upload,
account sharing, search, and folder/trash/restore capabilities.

## MinIO Upload Smoke

Run after the API is serving against docker-compose Postgres and MinIO:

```bash
docker compose up -d
cd services/api
DATABASE_URL=postgres://postgres:postgres@localhost:5433/drive_clone \
S3_ENDPOINT_URL=http://localhost:9000 \
S3_PUBLIC_ENDPOINT_URL=http://localhost:9000 \
S3_BUCKET=drive-clone \
S3_REGION=us-east-1 \
S3_URL_STYLE=path \
S3_ACCESS_KEY_ID=minioadmin \
S3_SECRET_ACCESS_KEY=minioadmin \
CORS_ALLOWED_ORIGINS=http://localhost:3000,http://127.0.0.1:3000 \
cargo run
```

If the web dev server uses another port, add that exact origin to `CORS_ALLOWED_ORIGINS`, for example `http://localhost:3100`.

Then in another terminal:

```bash
bash docs/evals/minio-upload-smoke.sh
```

The script signs up a user, creates a direct upload, PUTs bytes to MinIO, completes the file, verifies listing, requests a download URL, and byte-compares the downloaded object.

## Resumable Upload Smoke

Run after the API is serving against docker-compose Postgres and MinIO:

```bash
bash docs/evals/resumable-upload-smoke.sh
```

The script signs up a user, creates a resumable multipart upload, uploads the
first part, verifies the status endpoint reports that confirmed part, uploads the
remaining part, finalizes the multipart object, requests a download URL, and
byte-compares the downloaded object.

## Resumable Resume UI Smoke

Run after the API and web app are serving:

```bash
bash docs/evals/resumable-resume-ui-smoke.sh
```

The script creates a real resumable upload session through the API, seeds the
browser auth token plus pending upload record in `localStorage`, opens `/drive`,
clicks `Resume`, attaches the matching local file with a different modified
timestamp, and verifies the original upload session becomes `complete`.

## Password Reset Smoke

Run after the local compose stack is serving:

```bash
bash docs/evals/password-reset-smoke.sh
```

The script verifies the password reset contract: unknown email does not leak
account existence, a reset token changes the password, existing sessions are
revoked, the old password stops working, the new password works, and the token
cannot be reused.

## Share + Soft Delete Smoke

Runs against any live API (local stack or production):

```bash
API_BASE_URL=https://api-production-bcad4.up.railway.app bash docs/evals/share-delete-smoke.sh
```

Two fresh accounts exercise the full access-control flow: private-by-default download denial, share validations (unknown email 404, self-share 422, idempotent re-share), shared-with-me listing with owner info, grantee download with byte comparison, soft delete (owner list, trash, grantee access all react), restore, and revoke.

## Share Link Smoke

Runs against any live API (local stack or production):

```bash
API_BASE_URL=https://api-production-bcad4.up.railway.app bash docs/evals/share-link-smoke.sh
```

Two fresh accounts exercise public revocable links: non-owner create denial (404), non-positive expiry rejection (422), owner create returns a one-time token and `{PUBLIC_WEB_URL}/s/{token}` url, unauthenticated resolve with byte-compared download, owner list without the token, non-owner list denial, wrong-token 404, revoke then uniform 404, a 1-second link that lapses to 404, and a live link that 404s once the file is trashed.

## Folder Organization Smoke

Runs against any live API (local stack or production):

```bash
API_BASE_URL=https://api-production-bcad4.up.railway.app bash docs/evals/folder-organization-smoke.sh
```

Two fresh accounts exercise folder organization: create root and nested folders,
upload into a folder, browse root and folder contents, rename file and folder,
reject cross-user rename, reject a cyclic folder move, share the file, recursively
delete the folder tree, verify shared access disappears, restore the tree, byte
compare download, move the file back to root, and verify root browse.

## Search Smoke

Runs against any live API (local stack or production):

```bash
API_BASE_URL=https://api-production-bcad4.up.railway.app bash docs/evals/search-smoke.sh
```

Three fresh accounts exercise filename search ACLs: the owner sees owned active
files, a grantee sees only the file explicitly shared with them, a third user's
private file never appears, deleted files are excluded by default, and
`include_deleted=true` only exposes the owner's own trash.

## Sync Changes Smoke

Runs against any live API (local stack or production):

```bash
API_BASE_URL=https://api-production-bcad4.up.railway.app bash docs/evals/sync-changes-smoke.sh
```

Two fresh accounts exercise the `GET /sync/changes` feed: an owner uploads,
renames, and trashes a file, then verifies the feed returns upsert, upsert,
delete in gap-free seq order with a snapshot on upserts and only an id on the
tombstone; pagination with `limit=1` drains one entry at a time and reports
`has_more`; a cursor past the end returns empty without advancing; the other
user's feed never sees the owner's changes; and a negative cursor returns `422`.

## Worker Jobs Smoke

Run after docker-compose Postgres is available on port `5433`:

```bash
docker compose up -d postgres minio minio-create-bucket
bash docs/evals/worker-jobs-smoke.sh
```

The script runs the targeted integration tests for worker-owned behavior:
permanent trash purge deletes the bucket object and file row, decrements quota,
and preserves the sync tombstone; purge claims prevent duplicate workers from
processing the same file and block restore while permanent deletion is in
flight; stale claims cannot release or purge renewed claims; quota
reconciliation repairs a deliberately corrupted
`storage_used_bytes`.
