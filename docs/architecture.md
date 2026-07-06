# Architecture Notes

The first deployable service is the Rust API in `services/api`.

## Backend Layering

`services/api` is organized as a ports-and-adapters Rust service.

```text
services/api/src/
  domain/
  application/
  adapters/
  bootstrap/
```

### Domain

`domain` owns entities and rules that should be true regardless of transport or storage:

- user shape and credential validation
- password and token helpers
- file state and metadata models
- folder tree and breadcrumb models
- upload input validation
- quota calculation
- domain error categories

This layer must not import Axum, SQLx, Reqwest, environment variables, or object-storage adapters.

### Application

`application` owns use cases and ports:

- auth use cases: signup, login, logout, current user lookup
- file use cases: create upload, complete upload, browse folders, rename, move, list files, download file, trash, restore
- ports: auth repository, file repository, object storage, clock, ID generator

Use cases depend on traits, not concrete infrastructure. That keeps business behavior testable with in-memory fakes and without starting HTTP or PostgreSQL.

### Adapters

`adapters` owns concrete details:

- `http`: Axum DTOs, session extractor, route handlers, and error-to-HTTP mapping
- `postgres`: SQLx implementations for auth and file repositories
- `object_storage`: S3-compatible signing, disabled storage, and fake storage

Adapters implement application ports or translate external input/output into application requests and responses.

### Bootstrap

`bootstrap` is the dependency injection layer:

- reads runtime configuration
- connects to PostgreSQL
- runs SQLx migrations
- creates concrete repositories, storage adapters, clock, and ID generator
- wires use cases into `AppState`
- builds the router with CORS and tracing
- starts the Axum server

`main.rs` only calls `drive_clone_api::bootstrap::server::run().await`.

## Dependency Rule

The dependency direction is:

```text
bootstrap -> adapters -> application -> domain
```

Rules:

- `domain` imports no app/framework/database/storage crates.
- `application` imports `domain` and defines ports; it does not import Axum, SQLx, Reqwest, or environment variables.
- `adapters` import application ports and concrete crates.
- `bootstrap` wires concrete implementations together.

## Current File Flow

Upload and download keep the public contract in `contracts/files.md`:

1. `POST /files/uploads` authenticates the session, validates metadata, checks quota, signs a PUT URL, and stores a pending file row.
2. The browser uploads one full object directly to S3-compatible object storage.
3. `POST /files/{file_id}/complete` checks ownership, verifies object length with storage, marks the row complete, and increments storage usage once.
4. `GET /drive` lists active child folders and completed files in a root or folder location.
5. `GET /files/{file_id}/download` checks ownership and returns a short-lived signed GET URL.

Multipart/resumable upload sessions are intentionally outside the current file
flow. The next upload milestone adds session state, part tracking, status lookup,
finalization, expiration, and object cleanup without changing the owner-only
completion rule for visible files.

## Folder Tree Flow

Folders live in `folders` and files keep `parent_folder_id` in `files`.
`NULL` parent means My Drive root. This keeps the original file upload table
stable while adding a real user-owned tree.

Rules:

- Parent folders must be active and owned by the authenticated user.
- Rename validates the visible item name and does not touch object storage.
- Moving a file changes only `files.parent_folder_id`.
- Moving a folder changes `folders.parent_folder_id`.
- A recursive CTE rejects moving a folder into itself or a descendant.
- `GET /folders` returns a flat active folder list for move dialogs.
- `GET /drive?parent_folder_id=<uuid>` returns breadcrumbs, child folders, and files.

## Recursive Trash

File delete remains a single-row soft delete. Folder delete is a logical
recursive delete:

1. Lock the active owned root folder.
2. Use a recursive CTE to find active descendant folders.
3. Mark those folders and active descendant files deleted.
4. Set `deleted_by_folder_id` to the top folder id.

Restoring a folder restores only items marked by that folder delete. Files or
folders that were already individually deleted stay in trash. Restore fails if
the folder's parent is still deleted, because that would recreate an invalid
tree.
