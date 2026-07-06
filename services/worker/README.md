# Worker Service

The worker is the second binary in the `services/api` Rust crate. It reuses the
same migrations, Postgres repository, object-storage adapter, and file use cases
as the API.

Run locally:

```bash
cd services/api
DATABASE_URL=postgres://postgres:postgres@localhost:5433/drive_clone \
S3_ENDPOINT_URL=http://localhost:9000 \
S3_PUBLIC_ENDPOINT_URL=http://localhost:9000 \
S3_BUCKET=drive-clone \
S3_REGION=us-east-1 \
S3_URL_STYLE=path \
S3_ACCESS_KEY_ID=minioadmin \
S3_SECRET_ACCESS_KEY=minioadmin \
cargo run --bin drive-clone-worker
```

Docker Compose starts it as the `worker` service:

```bash
docker compose up worker
```

Jobs per tick:

- `expire_resumable_uploads`: expires stale pending resumable sessions and
  aborts multipart uploads.
- `purge_trash`: permanently deletes files in trash older than
  `TRASH_RETENTION_DAYS`, deletes the bucket object first, then removes the row
  and decrements quota. The worker claims rows with `purge_claimed_at` before
  touching object storage so multiple worker instances do not process the same
  file and users cannot restore a file once permanent deletion has started.
  Missing bucket objects are treated as already deleted.
- `cleanup_orphan_objects`: deletes bucket objects older than 24 hours that have
  no matching `files.object_key`.
- `reconcile_quota`: recomputes `users.storage_used_bytes` from complete files,
  including trashed files until permanent purge.

Environment:

- `DATABASE_URL`
- `S3_ENDPOINT_URL`
- `S3_PUBLIC_ENDPOINT_URL`
- `S3_BUCKET`
- `S3_REGION`
- `S3_URL_STYLE`
- `S3_ACCESS_KEY_ID`
- `S3_SECRET_ACCESS_KEY`
- `TRASH_RETENTION_DAYS` default `30`
- `WORKER_INTERVAL_SECONDS` default `300`
