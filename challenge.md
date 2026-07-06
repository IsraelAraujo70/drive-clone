# Google Drive Clone Challenge

## Context

You are going to design and build a cloud storage product inspired by Google Drive.

The goal is to practice software architecture, backend development, frontend product design, deployment, testing, and documentation. This is a portfolio challenge, so the final result should be understandable to another engineer, demonstrable in a browser, and supported by clear evidence that the hardest behaviors work.

Do not treat this challenge as a UI-only clone. The project must include real storage behavior, real metadata handling, real upload and download flows, and a credible architecture for scaling beyond a small demo.

## Product Goal

Build a web application where users can store, organize, download, share, and synchronize files.

The product should support a realistic first version while also explaining how the architecture could evolve toward a large-scale cloud storage system.

## Functional Requirements

### 1. File Upload

Users must be able to upload files to their personal drive.

The product should support:

- Uploading files from the web interface.
- Tracking upload progress.
- Rejecting uploads that exceed user or file-size limits.
- Preserving file metadata such as name, size, type, owner, location, and timestamps.
- Handling upload failures clearly.

### 2. File Download

Users must be able to download files from their drive.

The product should support:

- Downloading files owned by the current user.
- Downloading files shared with the current user.
- Preventing unauthorized file access.
- Returning the correct file bytes, filename, and content type.

### 3. File Synchronization

Users must be able to synchronize local files with the remote environment and vice versa.

The product should define:

- How clients discover remote changes.
- How creates, updates, moves, deletes, and restores are represented.
- How conflicts are handled when local and remote versions diverge.
- What information a future desktop or CLI sync client would need.

The first implementation may expose the synchronization contract before building a full desktop sync client.

### 4. File and Folder Management

Users must be able to organize their drive.

The product should support:

- Creating folders.
- Renaming files and folders.
- Moving files and folders.
- Deleting files and folders.
- Restoring deleted items.
- Preventing invalid folder structures.

### 5. Sharing

Users must be able to share files or folders with other people.

The product should define:

- What sharing modes are supported.
- How access is granted.
- How access is revoked.
- How private files remain private by default.

### 6. Search

Users must be able to search for files in their drive.

The product should support:

- Searching by filename.
- Returning useful metadata with results.
- Keeping search results scoped to files the user can access.
- Excluding deleted files unless the user is explicitly searching deleted items.

### 7. Resumable Uploads

Uploads must be resumable.

If the internet connection drops, the browser refreshes, or the client crashes, the system should be able to continue an upload from the last confirmed point instead of starting over from zero.

The product should define:

- How upload sessions are created.
- How partial progress is tracked.
- How the client discovers what still needs to be uploaded.
- How an upload is finalized.
- How abandoned uploads expire.

## Non-Functional Requirements

Design the system with these long-term scale targets in mind:

- 20 million registered users.
- 50 MB of free storage per user.
- Maximum file size of 50 MB per upload.
- 3 million uploads per day.
- Average upload size of 50 MB.

The system must also be designed for:

- High availability.
- Fault tolerance.
- Reliable upload recovery.
- Secure authorization.
- Clear observability.
- Deployability to a real cloud environment.

## Implementation Constraints

The project must use:

- Rust for the backend.
- TypeScript for the frontend.
- Railway as the preferred deployment target, unless a documented blocker makes another deployment target necessary.

The implementation should include a real database and real object/file storage. Do not store large file bytes directly in application memory or in a normal relational table as the primary storage mechanism.

## Expected Deliverables

The finished project should include:

- A deployed web application.
- Source code for the backend and frontend.
- A README explaining the chosen architecture and tradeoffs.
- Local setup instructions.
- Deployment instructions.
- API documentation.
- Database/schema documentation.
- Tests for core behaviors.
- Evaluation scenarios for resumable uploads, quota enforcement, sync behavior, and access control.
- A short demo script or demo video outline.

## Success Criteria

The challenge is successful when:

- A user can upload, view, organize, download, delete, restore, and share files.
- Upload limits and storage quota are enforced.
- Resumable uploads work after interruption.
- Private files cannot be accessed by unauthorized users.
- Search does not leak files across users.
- Synchronization behavior is specified and testable.
- The app is deployed or has a clear deploy path.
- The README explains the architecture well enough for another engineer to review it.
- The tests and evals provide evidence that the most important behaviors work.

## Current Implementation Status

Last updated: 2026-07-06.

Overall status: the web-drive MVP is implemented and locally verified. The
remaining work is no longer core CRUD/upload behavior; it is final product
completion around sync-client scope, folder sharing scope, production redeploy
and QA, and demo polish.

Status legend:

- Done: implemented, documented, and covered by automated tests or smoke evals.
- Partial: implemented enough for the portfolio MVP, but not the full long-term
  product behavior described in the challenge.
- Remaining: not implemented or not verified in the latest pushed state.

### Requirement Status

| Area | Status | Evidence | Remaining work |
| --- | --- | --- | --- |
| Authentication | Done | `POST /auth/signup`, `POST /auth/login`, `GET /auth/me`; API contract tests. | None for MVP. |
| File upload | Done | Resumable multipart upload is the primary UI path; direct upload remains for compatibility; API contract tests and `docs/evals/minio-upload-smoke.sh`. | Production smoke after the latest commits. |
| Upload progress | Done | Web upload progress and part progress in `apps/web/components/drive-shell.tsx`; web tests and resumable UI smoke. | Polish only. |
| Upload limits and quota | Done | `MAX_FILE_SIZE_BYTES=52428800`, `storage_quota_bytes=52428800`; quota tests and signup contract assertion. | Production migration/deploy verification after the latest quota change. |
| File download | Done | Authorized signed downloads for owner and grantee; byte-compare evals. | None for MVP. |
| Unauthorized access prevention | Done | Contract tests cover private file denial, cross-user access denial, deleted-file denial, share revocation, and public-link failure modes. | None for MVP. |
| File metadata | Done | PostgreSQL `files` and `folders` store name, owner, size, type, object key, state, timestamps, parent folder, delete state, and upload parts. | None for MVP. |
| Folder management | Done | Create, browse, rename, move, recursive trash, restore, invalid cycle rejection; `folder-organization-smoke.sh`. | None for MVP. |
| Trash restore | Done | File and folder restore from trash in API and UI. | None for MVP. |
| Force delete from trash | Done | UI `Force delete`; `DELETE /files/{file_id}/purge`; `DELETE /folders/{folder_id}/purge`; tests and real local smoke. | Production smoke after deploy. |
| User-to-user file sharing | Done | Owner shares by registered email, grantee sees shared-with-me and can download, owner can revoke. | None for MVP. |
| Public share links | Done | Create/list/revoke/resolve public links with expiry and uniform 404 on invalid/revoked/expired/trashed targets; `share-link-smoke.sh`. | None for MVP. |
| Folder sharing | Partial | Folder deletion/restore affects shared descendant file access correctly. | Directly sharing whole folders is not implemented. Decide whether the portfolio needs folder-level grants or whether file-level sharing is enough. |
| Search | Done | Filename search, ACL scoped, deleted excluded by default, owner's trash included only with `include_deleted=true`; `search-smoke.sh`. | None for MVP. |
| Sync change feed | Done for contract/API | `GET /sync/changes` emits upserts and tombstones with stable cursor and pagination; `sync-changes-smoke.sh`. | No desktop or CLI sync client yet. Conflict handling is specified at a high level, not implemented in a real client. |
| Resumable uploads | Done | Create session, upload parts, status endpoint, finalized multipart object, abandoned upload expiry, local resume UI using the same selected file; resumable tests and smoke evals. | Production resume smoke after latest deploy. |
| Cleanup jobs | Done | Worker handles expired resumable uploads, trash purge, orphan cleanup, and quota reconciliation; `worker-jobs-smoke.sh`. | Operational monitoring dashboards are not built. |
| Observability | Partial | Health endpoint, structured logs, worker job completion logs, eval scripts. | No metrics dashboard, alerting, or tracing backend. |
| Railway deployment | Partial | Railway-first config/docs exist and prior production upload/resume validation was performed. | Latest pushed changes still need Railway redeploy plus production smoke across upload, force delete, share, search, sync, and resume. |
| API documentation | Done | `docs/api/README.md`, `contracts/files.md`, and Bruno collection under `docs/api/bruno`. | Keep generated Bruno collection refreshed after route changes. |
| Database/schema documentation | Done | Migrations under `services/api/migrations`; architecture and API docs describe metadata model. | Optional ERD diagram. |
| Automated tests | Done | Rust unit tests, HTTP contract tests with real Postgres, web Vitest tests, Playwright e2e for theme and force delete. | Keep adding tests with feature changes. |
| Evals | Done | Product copy, upload, resumable upload, resume UI, share/delete, share link, folder organization, search, sync changes, worker jobs. | Run full eval suite against production after redeploy. |
| Demo script | Partial | README has manual demo flow and eval commands. | Write a short final demo script or record a demo video. |

### What Is Left To Finish

1. Redeploy the latest `main` to Railway and run production smoke tests.
   Required coverage: signup/login, resumable upload, resume UI, download,
   folder organization, trash restore, force delete, user sharing, public share
   link, search, sync changes, and worker jobs where production-safe.

2. Decide folder sharing scope.
   The current product shares files, not whole folders. If the challenge is read
   strictly as "files or folders", folder sharing should be added. If the
   portfolio story accepts file sharing plus public links, document that as the
   explicit v1 sharing mode.

3. Decide sync-client scope.
   The server-side sync feed is implemented and testable. A real desktop or CLI
   sync client is not built. The challenge allows exposing the sync contract
   first, so this is optional for MVP but the biggest remaining product gap.

4. Improve production observability.
   Add metrics/alerts or at least documented Railway log queries for upload
   errors, storage errors, worker failures, quota divergences, and share-link
   resolve failures.

5. Finalize demo material.
   Create a short demo script or video outline that follows the real happy path:
   create account, upload/resume, organize, share, search, delete, restore, force
   delete, and inspect sync changes.

6. Run the final acceptance pass.
   Run `make test`, all relevant `docs/evals/*.sh`, and browser QA on the
   deployed URL. Save the command outputs or screenshots as final evidence.
