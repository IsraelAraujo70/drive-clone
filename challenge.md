# Google Drive Clone Challenge

## Purpose and Learning Goals

Build a portfolio-grade Google Drive clone to practice product design, distributed systems, Rust backend development, TypeScript frontend development, and production deployment.

The goal is not to rebuild Google Drive at full production scale on day one. The goal is to design the system as if it could grow toward that scale, then implement a realistic and deployable version with clear milestones, tests, evals, and measurable outcomes.

This project should demonstrate:

- File upload, download, organization, and sharing workflows.
- Rust service design with explicit contracts and testable boundaries.
- TypeScript frontend design for a polished cloud storage experience.
- Resumable upload architecture for large files and unreliable networks.
- Storage quota enforcement and object storage integration.
- Deployment on Railway with a path to scale beyond the first deployment.
- Clear engineering documentation suitable for a software developer portfolio.

## Product Scope

The product is a web-based cloud storage application inspired by Google Drive. Users can create an account, upload files, organize files into folders, download files, share files, search metadata, and keep local state synchronized with the remote service through an API.

The first production version should feel complete for one real user managing personal files. The architecture should also document how the system would evolve to support millions of users, high upload volume, and fault-tolerant storage.

### In Scope

- User authentication and per-user storage ownership.
- A drive dashboard with files, folders, file metadata, and storage usage.
- Uploads and downloads through the web app.
- Resumable uploads for large files.
- Folder creation, rename, move, and delete.
- File rename, move, delete, and restore from trash.
- Sharing through private links or explicit user grants.
- Search by filename and basic metadata.
- A sync-oriented API that a future desktop or CLI client can use.
- Railway deployment for the backend, frontend, database, and object storage.

### Out of Scope for the MVP

- Real-time collaborative document editing.
- Google Workspace document rendering.
- Native mobile apps.
- Full desktop sync client.
- Enterprise admin features.
- Full-text indexing of arbitrary file contents.
- Multi-region production deployment.

These can become advanced portfolio extensions after the core drive experience is working.

## Functional Requirements

### 1. Upload Files

Users must be able to upload files from the browser into their drive.

The system must:

- Accept files through the web UI.
- Store file bytes in object storage.
- Store metadata in the database.
- Track filename, MIME type, size, owner, parent folder, checksum, upload status, and timestamps.
- Reject uploads that exceed the user's remaining quota.
- Reject files larger than the configured max file size.
- Show upload progress in the frontend.
- Preserve a clear failure state when upload completion cannot be confirmed.

Measurable outcome:

- A successful upload creates exactly one metadata record and one stored object.
- Failed uploads do not count against quota unless finalized.
- Upload success rate is tracked as an application metric.

### 2. Download Files

Users must be able to download files they own or are allowed to access.

The system must:

- Authorize every download.
- Stream downloads instead of loading entire files into application memory.
- Preserve the original filename and content type.
- Support signed object-storage URLs or backend-proxied downloads.
- Return clear errors for missing, deleted, or unauthorized files.

Measurable outcome:

- Authorized users can download the same bytes they uploaded.
- Unauthorized users cannot download private files.
- Download latency and error rate are tracked.

### 3. Synchronize Files

Users must be able to synchronize local file state with the remote environment through a sync API.

The MVP does not need to ship a full desktop sync client, but the backend must expose enough contract shape for one.

The system must:

- Track file and folder changes with monotonically ordered change records.
- Expose an API to list changes since a cursor.
- Return creates, updates, moves, deletes, and restores.
- Include file version or revision identifiers.
- Define conflict behavior when local and remote changes both modify the same item.

Default conflict rule:

- The server is the source of truth.
- If two edits conflict, preserve both versions and mark the client-uploaded version as a conflict copy.

Measurable outcome:

- A sync client can ask for changes since a known cursor and converge to the server state.
- Conflict behavior is deterministic and covered by tests.

### 4. Manage Files and Folders

Users must be able to organize files like a normal drive product.

The system must:

- Create folders.
- Rename files and folders.
- Move files and folders between folders.
- Soft-delete files and folders into trash.
- Restore files and folders from trash.
- Permanently delete trashed items.
- Prevent invalid folder trees, including cycles.

Measurable outcome:

- Folder operations preserve tree integrity.
- Deleted items disappear from the normal drive view but remain restorable until permanently deleted.

### 5. Share Files

Users must be able to share files and folders.

The system must support at least one of these sharing modes in the MVP:

- Private share link with a random unguessable token.
- Explicit share grant to another registered user.

The recommended MVP is private share links because it is simpler and easier to demonstrate in a portfolio demo.

The system must:

- Allow owners to create and revoke share links.
- Authorize shared downloads using the share token.
- Keep private files private by default.
- Track who owns the file and who has access.

Measurable outcome:

- A revoked share link stops working immediately.
- A private file without a share grant cannot be accessed by another user.

### 6. Search Files

Users must be able to search their drive by metadata.

The system must:

- Search by filename.
- Filter out deleted items unless the user searches trash.
- Scope results to the current user and permitted shared items.
- Return folder location and basic metadata.

Advanced search can later include MIME type, date ranges, size ranges, and full-text extraction.

Measurable outcome:

- Search results never leak another user's private files.
- Filename search returns expected results within an indexed query path.

### 7. Resumable Uploads

Uploads must be resumable so a network failure can continue from where it stopped.

The system must:

- Create an upload session before receiving chunks.
- Split large uploads into chunks.
- Track uploaded chunks.
- Allow the client to query upload session state.
- Resume from the next missing chunk.
- Finalize the file only after all chunks are present and validated.
- Expire abandoned upload sessions.

Measurable outcome:

- Killing the network or refreshing the browser during upload does not require restarting a completed chunk.
- Resume correctness is covered by tests and an eval scenario.

## Non-Functional Requirements

These requirements come from the target system-design challenge and should guide architecture decisions. They are design targets, not claims that the MVP will immediately serve this traffic.

### Scale Targets

- Support 20 million registered users as the long-term architecture target.
- Provide 15 GB of free storage per user.
- Allow each upload to be up to 15 GB.
- Support 3 million uploads per day with an average file size of 50 MB.

### Reliability Targets

- Uploads must be resumable.
- The system must operate with high availability.
- The system must be resilient and fault tolerant.
- Object storage failures must leave metadata in a recoverable state.
- Background processing must be retryable and idempotent.

### Performance Targets

- Metadata reads should be fast enough for interactive browsing.
- File bytes should stream directly to or from object storage when possible.
- Large uploads must not consume large backend memory.
- Search must use indexed database queries for MVP metadata search.

### Security Targets

- Every file operation must be scoped to the authenticated user or an explicit share grant.
- Object keys must not expose predictable user or filename data.
- Share tokens must be random and revocable.
- Passwords and secrets must never be logged.
- Uploaded files must be treated as untrusted data.

### Observability Targets

The system must leave evidence that it is working.

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

## Architecture Requirements

Use a services-first architecture with clear contracts between the frontend, backend, database, object storage, and background jobs.

### Backend

The backend must be written in Rust.

Recommended responsibilities:

- Authentication and session handling.
- File and folder metadata API.
- Upload session API.
- Download authorization.
- Share-link API.
- Search API.
- Sync API.
- Quota enforcement.
- Object storage coordination.
- Health and metrics endpoints.

The backend should expose a documented HTTP JSON API. File uploads may use multipart upload, chunked upload, or pre-signed object-storage operations, but the metadata state machine must remain owned by the backend.

### Frontend

The frontend must be written in TypeScript.

Recommended responsibilities:

- Login and signup screens.
- Drive file browser.
- Folder navigation.
- Upload progress UI.
- Resumable upload state.
- Download actions.
- Rename, move, delete, and restore actions.
- Share-link management.
- Search UI.
- Storage quota display.
- Error states for authorization, quota, network failure, and expired uploads.

The UI should prioritize a real product workflow over a marketing page. The first screen after login should be the usable drive interface.

### Metadata Database

Use PostgreSQL for durable metadata.

Minimum data concepts:

- Users.
- Files.
- Folders or drive items.
- Upload sessions.
- Uploaded chunks or parts.
- Share grants or share links.
- Change log entries for sync.

The database must enforce ownership, parent-child relationships, and enough constraints to prevent invalid drive state.

### Object Storage

Use S3-compatible object storage for file bytes.

The system must:

- Store file bytes outside the database.
- Generate opaque object keys.
- Keep metadata and object state consistent through explicit upload states.
- Support replacing the provider without changing the public API.

Railway buckets are the preferred first deployment target. If production-scale object storage requirements exceed Railway's practical limits, the same interface should allow migration to another S3-compatible provider.

### Resumable Upload State Machine

Uploads should move through explicit states:

- `created`
- `receiving`
- `finalizing`
- `complete`
- `failed`
- `expired`

Only `complete` files appear in the normal drive view and count as finalized stored files. Incomplete sessions can reserve temporary state, but they must not silently become visible files.

### Background Jobs

Use background jobs for work that should not block web requests.

Examples:

- Expire abandoned upload sessions.
- Clean up orphaned object-storage chunks.
- Recalculate user storage usage if reconciliation is needed.
- Generate previews or thumbnails in an advanced milestone.
- Process audit events or usage metrics.

Jobs must be retryable and safe to run more than once.

### Sync API

The sync API should be designed early even if the sync client is implemented later.

Minimum contract:

- `GET /sync/changes?cursor=<cursor>` returns ordered changes after the cursor.
- Each change includes item ID, change type, revision, timestamp, and relevant metadata.
- Deleted items appear as tombstones.
- A new cursor is returned with every response.

## Suggested Tech Stack

### Backend

- Rust.
- Axum for HTTP routing.
- Tokio for async runtime.
- SQLx for PostgreSQL access.
- Serde for JSON serialization.
- tower-http for tracing, CORS, and request middleware.
- aws-sdk-s3 or an S3-compatible Rust client for object storage.

### Frontend

- TypeScript.
- React.
- Vite.
- TanStack Query for server state.
- React Router for navigation.
- A small component system with accessible dialogs, menus, buttons, and tables.

### Data and Infrastructure

- PostgreSQL for metadata.
- Redis or a queue-backed worker for background jobs if needed.
- S3-compatible object storage for file bytes.
- Railway for deployment.
- GitHub Actions or local scripts for test and eval commands.

## Railway Deployment Plan

Railway is the preferred deployment target for the first hosted version.

Recommended Railway resources:

- Backend service: Rust API.
- Frontend service: TypeScript web app.
- PostgreSQL service: metadata database.
- Bucket: S3-compatible file storage.
- Optional Redis service or queue worker if background jobs need a separate process.

Required deployment behavior:

- `/health` endpoint returns healthy only when the API can reach required dependencies.
- Frontend uses a deployed API base URL.
- Backend reads all configuration from environment variables.
- Object storage credentials are injected through Railway variables.
- Database migrations run through a controlled command, not manually edited production state.
- Deploy status is validated after every release.

Production-scale note:

Railway is a good first deployment target for a portfolio app. The storage layer must remain S3-compatible so the object store can be replaced later without rewriting upload, download, or metadata APIs.

## Milestones

### Milestone 1: MVP Web Drive

Build the smallest complete cloud drive experience.

Deliver:

- Rust API service.
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

Done when:

- A user can sign up, upload a file, see it in the drive, download it, delete it, restore it, and see accurate storage usage.

### Milestone 2: Resumable Uploads

Add resumable upload sessions for large files and unstable networks.

Deliver:

- Upload session API.
- Chunk tracking.
- Resume status endpoint.
- Frontend upload resume behavior.
- Expiration cleanup job.
- Tests and evals for interrupted uploads.

Done when:

- A browser refresh or network interruption can resume a partially uploaded file without restarting completed chunks.

### Milestone 3: Sharing and Search

Add collaboration basics.

Deliver:

- Share-link creation and revocation.
- Shared download authorization.
- Filename search.
- Search indexes.
- Access-control tests.

Done when:

- Users can share a file through a link, revoke the link, and search their own drive without leaking private files.

### Milestone 4: Sync API and Rust Client

Add a sync-oriented backend contract and a small Rust CLI client.

Deliver:

- Change log table.
- Cursor-based sync API.
- Tombstone handling.
- Conflict-copy behavior.
- Rust CLI proof-of-concept sync client.

Done when:

- The CLI can sync a local folder from remote changes and handle a conflict deterministically.

### Milestone 5: Scale and HA Hardening

Turn the project into a strong system-design portfolio piece.

Deliver:

- Load tests for metadata APIs.
- Upload throughput tests.
- Quota reconciliation job.
- Object cleanup job.
- Metrics dashboard.
- Architecture diagram.
- Failure-mode documentation.
- Production-readiness checklist.

Done when:

- The repo can explain and demonstrate how the design moves from a portfolio deployment toward the stated scale targets.

## Testing and Evaluation

Every implementation milestone must include deterministic tests and eval scenarios.

### Gate Tests

Gate tests should be local, deterministic, and fast.

Required coverage:

- Quota enforcement.
- File ownership authorization.
- Share-link authorization and revocation.
- Folder tree integrity.
- Upload session state transitions.
- Chunk resume logic.
- Sync cursor ordering.
- Conflict behavior.
- Search scoping.

### Integration Tests

Integration tests should validate real service boundaries.

Required scenarios:

- Upload metadata plus object storage write.
- Download authorization plus object storage read.
- Delete and restore lifecycle.
- Expired upload cleanup.
- Database migration correctness.
- Railway-like environment configuration.

### Evals

Evals should test product behavior and architecture expectations.

Required eval scenarios:

- A 15 GB design review: confirm the upload path streams chunks and does not require loading the full file into memory.
- Resume correctness: interrupt an upload after several chunks, resume it, and verify final checksum.
- Quota behavior: fill an account near 15 GB and verify the next upload is rejected with a clear error.
- Sync convergence: apply remote changes, fetch from a cursor, and verify a client model reaches the server state.
- Access control: attempt cross-user file reads, downloads, and searches.
- Deploy health: verify the Railway deployment reports healthy and can perform a small upload/download smoke test.

### Load Tests

Load tests do not need to simulate 20 million real users during MVP, but they must document assumptions.

Measure:

- Metadata list latency.
- Search latency.
- Upload session creation rate.
- Chunk finalization rate.
- Download authorization latency.
- Database connection pool behavior.

## Portfolio Deliverables

The final portfolio version should include:

- Live deployed app URL.
- GitHub repository.
- README with local setup and deploy instructions.
- Architecture diagram.
- API documentation.
- Database schema documentation.
- Test and eval report.
- Demo video.
- Screenshots of core workflows.
- Short write-up explaining tradeoffs, scaling path, and failure modes.

Recommended demo script:

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

## Success Criteria

This challenge is complete when the project demonstrates both a working product and a credible architecture.

The product is successful if:

- A real user can manage files through the web app.
- Uploads and downloads work reliably.
- Resumable uploads recover from interruption.
- Storage quota is enforced.
- Sync behavior is deterministic.
- Sharing does not leak private files.
- Search stays scoped to authorized files.
- Railway deployment is healthy.

The portfolio is successful if:

- The README and challenge document explain what was built and why.
- The architecture diagram makes the system understandable in under two minutes.
- Tests and evals prove the hardest behaviors.
- The demo shows real workflows, not only static screens.
- The implementation choices are simple, documented, and defensible.
