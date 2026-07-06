# Drive Clone API

Base URLs:

- Local Docker/API: `http://localhost:8080`
- Production API: `https://api-production-bcad4.up.railway.app`

The API is a Rust/Axum service under `services/api`. All request and response
bodies are JSON unless noted. Authenticated endpoints use:

```http
Authorization: Bearer <token>
```

Errors use one shape:

```json
{ "error": "<machine_code>", "message": "<safe UI message>" }
```

Stable error codes:

- `validation_error`
- `unauthorized`
- `invalid_credentials`
- `email_taken`
- `quota_exceeded`
- `file_too_large`
- `file_not_found`
- `invalid_file_state`
- `storage_error`
- `user_not_found`
- `internal_error`

## Bruno Collection

The Git-friendly Bruno collection lives at:

```text
docs/api/bruno
```

Open it in Bruno, select `Development` or `Production`, then run:

1. `Auth / Signup` or `Auth / Login`
2. `Files / Create Upload`
3. `Files / Direct PUT Upload URL`
4. `Files / Complete Upload`
5. Any file, folder, share, search, or trash endpoint

Useful variables:

- `baseUrl`
- `authToken`
- `userId`
- `fileId`
- `folderId`
- `granteeId`
- `granteeEmail`
- `uploadUrl`
- `objectKey`
- `searchQuery`
- `checksumSha256`

Regenerate the collection after route changes:

```bash
node scripts/generate-bruno-collection.mjs
```

## Objects

### User

```json
{
  "id": "uuid",
  "email": "user@example.com",
  "display_name": "User Name",
  "storage_quota_bytes": 16106127360,
  "storage_used_bytes": 0,
  "created_at": "2026-07-02T18:00:00Z"
}
```

### File

```json
{
  "id": "uuid",
  "filename": "report.pdf",
  "parent_folder_id": null,
  "content_type": "application/pdf",
  "size_bytes": 12345,
  "checksum_sha256": null,
  "object_key": "owner/file",
  "state": "complete",
  "created_at": "2026-07-02T18:00:00Z",
  "updated_at": "2026-07-02T18:01:00Z",
  "completed_at": "2026-07-02T18:01:00Z",
  "deleted_at": null
}
```

### Folder

```json
{
  "id": "uuid",
  "name": "Projects",
  "parent_folder_id": null,
  "created_at": "2026-07-06T12:00:00Z",
  "updated_at": "2026-07-06T12:00:00Z",
  "deleted_at": null
}
```

### Share User

```json
{
  "id": "uuid",
  "email": "friend@example.com",
  "display_name": "Friend"
}
```

## System

### `GET /`

Returns the service identity.

Response `200`:

```json
{
  "service": "drive-clone-api",
  "message": "Google Drive clone API"
}
```

### `GET /health`

Checks database reachability.

Responses:

- `200`: `{ "status": "ok", "service": "drive-clone-api" }`
- `503`: `{ "status": "degraded", "service": "drive-clone-api" }`

## Auth

### `POST /auth/signup`

Creates an account and session.

Request:

```json
{
  "email": "user@example.com",
  "password": "password123",
  "display_name": "User Name"
}
```

Rules:

- Email is trimmed and lowercased.
- Password length: 8-128 chars.
- Display name length after trim: 1-100 chars.

Responses:

- `201`: `{ "user": User, "token": "opaque-token" }`
- `409 email_taken`
- `422 validation_error`

### `POST /auth/login`

Authenticates an existing user.

Request:

```json
{
  "email": "user@example.com",
  "password": "password123"
}
```

Responses:

- `200`: `{ "user": User, "token": "opaque-token" }`
- `401 invalid_credentials`

### `POST /auth/logout`

Authenticated. Deletes the current server-side session.

Responses:

- `204`
- `401 unauthorized`

### `GET /auth/me`

Authenticated. Returns the current user.

Responses:

- `200`: `User`
- `401 unauthorized`

## Drive And Search

### `GET /drive`

Authenticated. Browses My Drive root.

Response `200`:

```json
{
  "parent_folder_id": null,
  "breadcrumbs": [],
  "folders": [Folder],
  "files": [File]
}
```

### `GET /drive?parent_folder_id=<uuid>`

Authenticated. Browses an active folder owned by the current user.

Responses:

- `200`: same shape as `GET /drive`
- `404 file_not_found` for missing, deleted, or cross-user folders

### `GET /search?q=<query>&include_deleted=false&limit=20`

Authenticated. Searches completed files by filename.

Rules:

- `q`: required after trim, 1-100 chars.
- `include_deleted`: optional, default `false`.
- `limit`: optional, default `20`, clamped to `1..50`.
- Scope: owned active files plus active files shared with the user.
- `include_deleted=true` also includes deleted owned files.
- Deleted shared files never appear.
- Matching is case-insensitive substring search.
- Literal `%`, `_`, and `\` are treated as normal text.

Response `200`:

```json
{
  "query": "report",
  "files": [
    {
      "access": "owned",
      "file": File,
      "owner": null
    },
    {
      "access": "shared",
      "file": File,
      "owner": ShareUser
    }
  ]
}
```

Ordering:

1. Exact filename match.
2. Prefix match.
3. Substring match.
4. `completed_at DESC`.
5. `id DESC`.

### `GET /drive/trash`

Authenticated. Returns top-level trash entries owned by the user.

Response `200`:

```json
{
  "parent_folder_id": null,
  "breadcrumbs": [],
  "folders": [Folder],
  "files": [File]
}
```

## Sync

### `GET /sync/changes?cursor=<seq>&limit=<n>`

Authenticated. Per-owner, cursor-based change feed with tombstones. Every file
and folder mutation appends a gap-free, monotonic entry under the owner's
`change_seq`, so a client can persist one integer and poll for deltas.

Query:

- `cursor` (optional, default `0`): last consumed `seq`. First call sends `0`.
  Negative → `422 validation_error`.
- `limit` (optional, default `100`, clamped to `1..=500`).

Behavior:

- `op: "upsert"` covers create, upload completion, rename, move, and restore and
  embeds the current entity snapshot (`file` or `folder`).
- `op: "delete"` is a tombstone (soft delete / trash), carrying only
  `entity_id`. Restoring emits a later `upsert`.
- Trashing a folder emits one `delete` per node in the subtree.
- Pending uploads never appear. Changes are isolated per owner.

Response `200`:

```json
{
  "changes": [
    {
      "seq": 42,
      "entity_type": "file",
      "op": "upsert",
      "entity_id": "uuid",
      "occurred_at": "2026-07-06T12:00:00Z",
      "file": {
        "id": "uuid",
        "filename": "report.pdf",
        "parent_folder_id": null,
        "size_bytes": 12345,
        "content_type": "application/pdf",
        "checksum_sha256": null,
        "updated_at": "2026-07-06T12:00:00Z"
      }
    },
    {
      "seq": 43,
      "entity_type": "folder",
      "op": "delete",
      "entity_id": "uuid",
      "occurred_at": "2026-07-06T12:00:01Z"
    }
  ],
  "next_cursor": 43,
  "has_more": false
}
```

`next_cursor` is the last returned `seq` (or the request cursor when empty).
`has_more` is `true` while `changes.len() == limit`; page until it is `false`.

## Folders

### `POST /folders`

Authenticated. Creates a folder in root or an active owned parent folder.

Request:

```json
{
  "name": "Projects",
  "parent_folder_id": null
}
```

Rules:

- Name is trimmed.
- Name max length is 255 chars.
- Name cannot be empty, `.`, `..`, or contain `/` or `\`.
- Duplicate sibling names are allowed.

Responses:

- `201`: `Folder`
- `404 file_not_found` for invalid parent
- `422 validation_error`

### `GET /folders`

Authenticated. Returns all active folders owned by the user as a flat list.

Response `200`:

```json
{ "folders": [Folder] }
```

### `PATCH /folders/{folder_id}`

Authenticated owner-only rename and/or move.

Request:

```json
{
  "name": "Renamed",
  "parent_folder_id": null
}
```

Rules:

- At least one of `name` or `parent_folder_id` must be provided.
- `parent_folder_id: null` moves the folder to root.
- Moving into itself or a descendant returns `409 invalid_file_state`.

Responses:

- `200`: `Folder`
- `404 file_not_found`
- `409 invalid_file_state`
- `422 validation_error`

### `DELETE /folders/{folder_id}`

Authenticated owner-only recursive soft delete.

Responses:

- `204`
- `404 file_not_found`

### `POST /folders/{folder_id}/restore`

Authenticated owner-only recursive restore for a folder tree deleted by that
folder delete.

Responses:

- `200`: `Folder`
- `404 file_not_found`
- `409 invalid_file_state` when parent remains deleted

## Files

Current upload scope: new clients should use resumable multipart uploads. The
original direct single-object upload endpoint remains supported for compatibility
and simple smoke tests.

### `POST /files/uploads`

Authenticated. Compatibility direct upload path. Creates pending file metadata
and returns a presigned PUT URL for one full-object upload.

Request:

```json
{
  "filename": "report.pdf",
  "parent_folder_id": null,
  "content_type": "application/pdf",
  "size_bytes": 12345,
  "checksum_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}
```

Rules:

- Filename follows folder-name validation.
- `parent_folder_id` must be an active owned folder when present.
- `content_type` must look like `type/subtype`.
- `size_bytes` must be positive, within max file size, and within quota.
- `checksum_sha256` is optional lowercase SHA-256 hex.

Response `201`:

```json
{
  "file_id": "uuid",
  "upload_url": "https://...",
  "object_key": "owner/file",
  "expires_at": "2026-07-02T18:15:00Z"
}
```

### Direct `PUT upload_url`

Not an API route. The client uploads raw bytes directly to object storage using
the `upload_url` from `POST /files/uploads`. This is one full-object PUT for the
current contract.

Required header:

```http
Content-Type: <same content_type used in metadata>
```

### `POST /files/{file_id}/complete`

Authenticated owner-only. Verifies object length and marks a pending file
complete.

Responses:

- `200`: `File`
- `404 file_not_found`
- `409 invalid_file_state`
- `502 storage_error`

### `POST /files/uploads/resumable`

Authenticated. Creates a pending resumable multipart upload session and starts
the backing object-storage multipart upload.

Request:

```json
{
  "filename": "video.mov",
  "parent_folder_id": null,
  "content_type": "video/quicktime",
  "size_bytes": 6291493,
  "checksum_sha256": null,
  "part_size_bytes": 6291456
}
```

Rules:

- Same validation, parent-folder ownership, max-size, and quota rules as direct
  upload creation.
- `part_size_bytes` is optional. Default is 8 MiB. The server raises too-small
  values to satisfy S3-compatible multipart limits.
- Sessions expire at `expires_at`.

Response `201`:

```json
{
  "file_id": "uuid",
  "object_key": "owner/object",
  "part_size_bytes": 8388608,
  "expires_at": "2026-07-06T19:30:00Z"
}
```

### `GET /files/uploads/{file_id}/status`

Authenticated owner-only. Returns confirmed resumable upload progress.

Response `200`:

```json
{
  "file_id": "uuid",
  "filename": "video.mov",
  "parent_folder_id": null,
  "content_type": "video/quicktime",
  "size_bytes": 6291493,
  "checksum_sha256": null,
  "object_key": "owner/object",
  "state": "pending",
  "part_size_bytes": 6291456,
  "expires_at": "2026-07-06T19:30:00Z",
  "created_at": "2026-07-06T19:15:00Z",
  "updated_at": "2026-07-06T19:15:00Z",
  "completed_at": null,
  "parts": [
    { "part_number": 1, "size_bytes": 6291456, "etag": "\"etag-1\"" }
  ]
}
```

### `GET /files/uploads/pending`

Authenticated. Lists the caller's own resumable sessions that are still `pending`
and not expired, so the web client can recover interrupted uploads. The server is
authoritative for existence and expiration.

Response `200`:

```json
{
  "uploads": [
    {
      "file_id": "uuid",
      "filename": "video.mov",
      "parent_folder_id": null,
      "size_bytes": 6291493,
      "part_size_bytes": 6291456,
      "checksum_sha256": null,
      "parts_received": 1,
      "expires_at": "2026-07-06T19:30:00Z"
    }
  ]
}
```

### `POST /files/uploads/{file_id}/parts`

Authenticated owner-only. Signs a direct object-storage PUT URL for one part.

Request:

```json
{ "part_number": 1 }
```

Response `200`:

```json
{
  "file_id": "uuid",
  "part_number": 1,
  "upload_url": "https://...",
  "expires_at": "2026-07-06T19:30:00Z",
  "expected_size_bytes": 6291456
}
```

The client uploads exactly `expected_size_bytes` bytes to `upload_url` and reads
the object-storage `ETag` response header.

### `POST /files/uploads/{file_id}/parts/{part_number}`

Authenticated owner-only. Records a successfully uploaded part.

Request:

```json
{ "size_bytes": 6291456, "etag": "\"etag-1\"" }
```

Response `200`:

```json
{ "part_number": 1, "size_bytes": 6291456, "etag": "\"etag-1\"" }
```

### `POST /files/uploads/{file_id}/finalize`

Authenticated owner-only. Completes the object-storage multipart upload, verifies
the final object size, marks the file complete, and increments storage usage
once.

Responses:

- `200`: `File`
- `404 file_not_found`
- `409 invalid_file_state`
- `502 storage_error`

### `POST /files/uploads/cleanup-expired`

Authenticated. Expires and aborts up to 100 stale pending resumable uploads.

Response `200`:

```json
{ "expired_count": 2, "aborted_count": 2 }
```

### `GET /files`

Authenticated. Lists completed active files owned by the user.

Response `200`:

```json
{ "files": [File] }
```

### `GET /files/{file_id}/download`

Authenticated. Returns a presigned GET URL when the current user owns the file
or has a current share grant.

Response `200`:

```json
{
  "download_url": "https://...",
  "expires_at": "2026-07-02T18:15:00Z"
}
```

### `PATCH /files/{file_id}`

Authenticated owner-only rename and/or move for a complete active file.

Request:

```json
{
  "filename": "renamed.pdf",
  "parent_folder_id": null
}
```

Responses:

- `200`: `File`
- `404 file_not_found`
- `422 validation_error`

### `DELETE /files/{file_id}`

Authenticated owner-only soft delete.

Responses:

- `204`
- `404 file_not_found`

### `POST /files/{file_id}/restore`

Authenticated owner-only restore for a directly deleted file.

Responses:

- `200`: `File`
- `404 file_not_found`

### `GET /files/trash`

Authenticated. Lists deleted files owned by the user.

Response `200`:

```json
{ "files": [File] }
```

### `GET /files/shared-with-me`

Authenticated. Lists active complete files shared with the current user.

Response `200`:

```json
{
  "files": [
    {
      "...File": "...",
      "owner": ShareUser
    }
  ]
}
```

## Shares

Sharing is file-level view/download permission by registered user email. Files
are private by default. Public tokenized links are documented under
[Share Links](#share-links) below.

### `POST /files/{file_id}/shares`

Authenticated owner-only.

Request:

```json
{ "email": "friend@example.com" }
```

Rules:

- File must be complete and active.
- Unknown email returns `404 user_not_found`.
- Self-share returns `422 validation_error`.
- Re-sharing with the same user is idempotent.

Response `201`:

```json
{
  "file_id": "uuid",
  "grantee": ShareUser,
  "created_at": "2026-07-03T12:00:00Z"
}
```

### `GET /files/{file_id}/shares`

Authenticated owner-only. Lists current grantees.

Response `200`:

```json
{ "shares": [ { "file_id": "uuid", "grantee": ShareUser, "created_at": "..." } ] }
```

### `DELETE /files/{file_id}/shares/{grantee_id}`

Authenticated owner-only. Revokes a grantee's access.

Responses:

- `204`
- `404 file_not_found`

## Share Links

Read-only, revocable public links to a single file. The token is 32 random
bytes (base64url); only its SHA-256 hash is stored. The plaintext token is
returned once, in the creation response. Links reuse the existing presigned
download flow, so they need no authentication to download.

### `POST /files/{file_id}/share-links`

Authenticated owner-only. File must be complete and active.

Request (`expires_in_seconds` is optional; `null` or omitted → never expires):

```json
{ "expires_in_seconds": 604800 }
```

Rules:

- Non-owner / unknown / incomplete / deleted file → `404 file_not_found`.
- `expires_in_seconds` present and `<= 0` → `422 validation_error`.

Response `201`:

```json
{
  "id": "uuid",
  "token": "urlsafe-token",
  "url": "https://app.example.com/s/urlsafe-token",
  "expires_at": "2026-07-13T12:00:00Z"
}
```

`url` = `{PUBLIC_WEB_URL}/s/{token}`. `expires_at` is `null` for a
non-expiring link.

### `GET /files/{file_id}/share-links`

Authenticated owner-only. Lists the file's links without tokens.

Response `200`:

```json
{ "links": [ { "id": "uuid", "created_at": "...", "expires_at": "...", "revoked_at": null } ] }
```

### `DELETE /files/{file_id}/share-links/{link_id}`

Authenticated owner-only. Sets `revoked_at` (row kept for audit).

Responses:

- `204`
- `404 file_not_found`

### `GET /shared/links/{token}`

Public, unauthenticated. Resolves a valid link. Any failure mode (bad token,
revoked, expired, trashed file) returns a uniform `404 file_not_found`.

Response `200`:

```json
{
  "filename": "report.pdf",
  "size_bytes": 12345,
  "content_type": "application/pdf",
  "download_url": "https://storage.example/objects/...presigned..."
}
```

## Future API Contracts

- Share links: password protection, folder links, and write-capable links are
  out of scope for the current read-only single-file link.

## Validation Commands

```bash
node scripts/generate-bruno-collection.mjs
DATABASE_URL=postgres://postgres:postgres@localhost:5433/drive_clone cargo test --manifest-path services/api/Cargo.toml
npm test --prefix apps/web
```
