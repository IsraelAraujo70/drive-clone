# Google Drive Clone

This repository is the answer to the [Google Drive Clone Challenge](./challenge.md): a portfolio-grade cloud storage application built with a Rust backend, a TypeScript frontend, and a Railway-first deployment target.

The goal is to build a real product experience first, then document how the system can grow toward the larger system-design targets from the challenge.

## Product Direction

The first version should be a working personal cloud drive:

- Sign up and log in.
- Upload files.
- Browse files and folders.
- Download files.
- Rename, move, delete, and restore items.
- See storage usage.
- Share files through revocable links.
- Search files by name.
- Resume interrupted uploads.

The MVP should feel usable by one real person. The architecture should still be explicit about how it would scale toward 20 million registered users, 15 GB of free storage per user, 15 GB max files, and 3 million uploads per day.

## Proposed Architecture

Use a services-first architecture:

- `apps/web`: TypeScript frontend.
- `services/api`: Rust HTTP API.
- `services/worker`: Rust background worker, added when cleanup and async jobs are needed.
- `contracts`: shared API and schema documentation.
- `docs`: architecture diagrams, deployment notes, and eval reports.

The backend owns metadata, authorization, upload state, quota enforcement, and sync semantics. File bytes live in S3-compatible object storage. PostgreSQL stores durable metadata.

## Repository Structure

Current structure:

```text
apps/
  web/
contracts/
docs/
infra/
  railway/
services/
  api/
  worker/
```

Current deployable service:

- `services/api`: minimal Rust API scaffold with `/` and `/health`.

Run locally:

```bash
cd services/api
cargo test
PORT=8080 cargo run
curl http://127.0.0.1:8080/health
```

### Core Components

Backend API:

- Authentication and sessions.
- File and folder metadata.
- Upload sessions.
- Download authorization.
- Sharing.
- Search.
- Sync cursor API.
- Quota enforcement.
- Health checks and metrics.

Frontend:

- Login and signup screens.
- Drive browser.
- Folder navigation.
- Upload progress and resume UI.
- File actions.
- Trash and restore flows.
- Share-link management.
- Search.
- Storage usage display.

Data stores:

- PostgreSQL for metadata.
- S3-compatible object storage for file bytes.
- Redis or a queue only if background jobs need it.

## Suggested Tech Stack

Backend:

- Rust.
- Axum for HTTP routing.
- Tokio for async runtime.
- SQLx for PostgreSQL.
- Serde for JSON.
- tower-http for tracing, CORS, and middleware.
- An S3-compatible Rust client for object storage.

Frontend:

- TypeScript.
- React.
- Next.js (App Router).
- Tailwind CSS.
- shadcn/ui.

Infrastructure:

- Railway backend service.
- Railway frontend service.
- Railway PostgreSQL.
- Railway S3-compatible bucket for the first deployment.
- Optional Railway Redis or worker service for background jobs.

The storage interface must stay S3-compatible so the project can move from Railway buckets to another object storage provider later without changing the public API.

## Data Model

Minimum metadata concepts:

- `users`: account identity and storage quota.
- `drive_items`: files and folders in a user-owned tree.
- `files`: file-specific metadata, object key, checksum, size, and upload state.
- `upload_sessions`: resumable upload lifecycle.
- `upload_parts`: confirmed chunks or parts for resumable uploads.
- `share_links`: revocable private sharing tokens.
- `change_log`: ordered events for sync clients.

Only completed files should appear in the normal drive view. Incomplete uploads should remain visible only through upload-session APIs.

## Upload Design

Uploads should use an explicit state machine:

- `created`
- `receiving`
- `finalizing`
- `complete`
- `failed`
- `expired`

Recommended flow:

1. Client creates an upload session with filename, size, parent folder, and checksum metadata.
2. API checks quota and file-size limits.
3. Client uploads chunks or object-storage parts.
4. API records confirmed parts.
5. Client asks the API to finalize.
6. API validates all expected parts, writes final metadata, and marks the file `complete`.

Measurable outcomes:

- Failed uploads do not become visible files.
- Completed uploads produce one metadata record and one stored object.
- Resume state can be queried deterministically.
- The backend never needs to load a 15 GB file into memory.

## Download Design

Every download must be authorized before bytes are returned.

The API can either:

- Return a short-lived signed object-storage URL.
- Proxy the stream from object storage.

The signed URL path is preferred for scale because the application service avoids streaming large files through its own compute layer.

Measurable outcomes:

- Authorized users can download exactly the bytes they uploaded.
- Unauthorized users cannot download private files.
- Download errors are tracked by reason.

## Sharing Design

The MVP should use private share links:

- Owners can create a random share token.
- Owners can revoke the token.
- Downloads through the token still pass through authorization logic.
- Files remain private by default.

Explicit user-to-user sharing can be added after link sharing works.

Measurable outcomes:

- Revoked links stop working immediately.
- Search and browse endpoints do not expose private files to other users.

## Sync Design

Expose a cursor-based sync API early, even before a full desktop client exists.

Minimum endpoint:

```http
GET /sync/changes?cursor=<cursor>
```

Each change should include:

- Item ID.
- Change type.
- Revision.
- Timestamp.
- Parent folder.
- Relevant metadata.
- Tombstone data for deletes.

Conflict default:

- The server is the source of truth.
- If local and remote edits conflict, preserve both versions.
- The client-uploaded conflicting version becomes a conflict copy.

Measurable outcomes:

- A client can fetch changes since a cursor and converge to server state.
- Conflict behavior is deterministic and tested.

## Railway Deployment Plan

Recommended Railway resources:

- Backend service: Rust API.
- Frontend service: TypeScript web app.
- PostgreSQL service: metadata database.
- Bucket: S3-compatible file storage.
- Optional worker service: cleanup and async jobs.
- Optional Redis service: queue or short-lived coordination.

Required deployment behavior:

- `/health` returns healthy only when required dependencies are reachable.
- Frontend reads the deployed API base URL from configuration.
- Backend reads all configuration from environment variables.
- Object storage credentials are injected through Railway variables.
- Database migrations run through a controlled command.
- Every release gets a smoke test: sign in, upload a small file, download it, delete it, restore it.

Initial deployment status:

- Project: `drive-clone`.
- API URL: `https://api-production-bcad4.up.railway.app`.
- Web URL: `https://web-production-c3311.up.railway.app`.
- API service source: `IsraelAraujo70/drive-clone`, branch `google-drive-clone-challenge`, root `/services/api`.
- Web service source: `IsraelAraujo70/drive-clone`, branch `google-drive-clone-challenge`, root `/apps/web`.
- Latest verified API deployment: `86164b07-f605-4724-9efb-32e3464baf9c`.
- Latest verified web deployment: `ef2f4843-f122-400b-92ff-ec0157329d67`.
- Product resources for Postgres, buckets, and workers will be added as their implementation lands.

## Milestones

### Milestone 1: MVP Web Drive

Deliver:

- Rust API.
- TypeScript web app.
- User auth.
- File upload.
- File download.
- Folder browsing.
- Rename, move, delete, and restore.
- Storage quota display.
- PostgreSQL metadata.
- S3-compatible object storage.
- Railway deployment.

Done when a user can sign up, upload a file, see it in the drive, download it, delete it, restore it, and see accurate storage usage.

### Milestone 2: Resumable Uploads

Deliver:

- Upload session API.
- Chunk tracking.
- Resume status endpoint.
- Frontend resume behavior.
- Expiration cleanup job.
- Tests and evals for interrupted uploads.

Done when a browser refresh or network interruption can resume a partially uploaded file without restarting completed chunks.

### Milestone 3: Sharing and Search

Deliver:

- Share-link creation and revocation.
- Shared download authorization.
- Filename search.
- Search indexes.
- Access-control tests.

Done when users can share a file through a link, revoke the link, and search their own drive without leaking private files.

### Milestone 4: Sync API and Rust Client

Deliver:

- Change log table.
- Cursor-based sync API.
- Tombstone handling.
- Conflict-copy behavior.
- Rust CLI proof-of-concept sync client.

Done when the CLI can sync a local folder from remote changes and handle conflicts deterministically.

### Milestone 5: Scale and Reliability Hardening

Deliver:

- Load tests for metadata APIs.
- Upload throughput tests.
- Quota reconciliation job.
- Object cleanup job.
- Metrics dashboard.
- Architecture diagram.
- Failure-mode documentation.
- Production-readiness checklist.

Done when the repo can explain and demonstrate how the design moves from portfolio deployment toward the stated scale targets.

## Tests and Evals

Gate tests should be deterministic, local, fast, and run on every meaningful change.

Required gate test coverage:

- Quota enforcement.
- File ownership authorization.
- Share-link authorization and revocation.
- Folder tree integrity.
- Upload session state transitions.
- Chunk resume logic.
- Sync cursor ordering.
- Conflict behavior.
- Search scoping.

Integration tests should cover:

- Upload metadata plus object storage write.
- Download authorization plus object storage read.
- Delete and restore lifecycle.
- Expired upload cleanup.
- Database migration correctness.
- Railway-like environment configuration.

Eval scenarios should cover:

- 15 GB upload design review: the upload path streams chunks and never requires the full file in memory.
- Resume correctness: interrupt after several chunks, resume, and verify final checksum.
- Quota behavior: fill an account near 15 GB and reject the next upload with a clear error.
- Sync convergence: apply remote changes, fetch from a cursor, and verify client state.
- Access control: attempt cross-user reads, downloads, and searches.
- Deploy health: verify Railway health and a small upload/download smoke test.

## Observability

Track:

- Upload success rate.
- Upload failure rate by reason.
- Resume success rate.
- Download success and error rates.
- Storage used per user.
- Quota rejection count.
- Share-link access count.
- Background job retry count.
- Deployment health checks.

## Demo Script

1. Sign up.
2. Upload a file.
3. Create a folder.
4. Move the file into the folder.
5. Download the file.
6. Delete and restore the file.
7. Start a large upload, interrupt it, and resume it.
8. Create and revoke a share link.
9. Search for the file.
10. Show deployment health and test results.

## Local Development

Requirements: Docker with Compose Watch support. Rust and Node are only required
when running services directly on the host.

```bash
# Full stack with file watching:
make dev

# Full stack in the background:
make up

# URLs:
# Web: http://localhost:3000
# API: http://localhost:8080/health
# MinIO console: http://localhost:9001
```

`make dev` runs `docker compose up --watch --build`. The web container syncs
source files into the Next.js dev server for hot reload. The API container syncs
Rust source and migration files, then restarts `cargo run` so code changes
recompile inside the container. Dependency manifest changes rebuild the affected
image.

Useful commands:

```bash
make logs
make ps
make down
make test
make eval-upload
make clean
```

Direct host-run commands still work when needed:

```bash
# 1. Start only the dependencies
docker compose up -d postgres minio minio-create-bucket

# 2. Run the API (migrations run automatically on boot)
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

# 3. Run the web app (in another terminal)
cd apps/web
npm install
npm run dev
```

Environment examples live in `services/api/.env.example` and `apps/web/.env.example`.
If the web dev server uses another port, add that exact origin to `CORS_ALLOWED_ORIGINS`, for example `http://localhost:3100`.

### Tests

```bash
# API: unit + integration tests (integration tests need the compose Postgres up)
cd services/api
DATABASE_URL=postgres://postgres:postgres@localhost:5433/drive_clone cargo test

# Upload smoke against the running API and MinIO
cd ../..
bash docs/evals/minio-upload-smoke.sh

# Web: vitest
cd apps/web
npm test
```

## Current Status

Implemented so far:

- Landing page, signup, and login (English UI) with a protected `/drive` shell, built on Next.js + Tailwind CSS + shadcn/ui.
- Rust API on Axum + SQLx + PostgreSQL: `POST /auth/signup`, `POST /auth/login`, `POST /auth/logout`, `GET /auth/me`, and a DB-aware `GET /health`.
- File upload backend: `POST /files/uploads`, `POST /files/{file_id}/complete`, `GET /files`, and `GET /files/{file_id}/download`.
- Argon2 password hashing; opaque bearer session tokens stored hashed (SHA-256) with 30-day expiry.
- Auth contract in `contracts/auth.md`; files contract in `contracts/files.md`; migrations in `services/api/migrations`.
- Gate tests: API validation/token tests plus full HTTP auth/file flows against real Postgres, and web tests.
- Repo-connected Railway deployments for the API and web services.

Next milestone: file upload, folder browsing, and download (Milestone 1 continues).
