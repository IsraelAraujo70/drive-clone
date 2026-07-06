# Files Contract

Base URL: the API service root. All endpoints require:

```http
Authorization: Bearer <token>
```

Error shape:

```json
{ "error": "<machine_code>", "message": "<safe UI message>" }
```

Stable file error codes:

- `validation_error`
- `unauthorized`
- `quota_exceeded`
- `file_too_large`
- `file_not_found`
- `invalid_file_state`
- `storage_error`
- `user_not_found`

Folder organization uses the same `file_not_found` code for private or missing
folders so callers cannot distinguish cross-user resources from absent ones.
Invalid folder moves, including cycles, return `invalid_file_state`.

Current upload scope: this contract supports both the original direct
single-object upload and the resumable multipart upload flow. New clients should
prefer resumable uploads.

## POST /files/uploads

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

- `filename` is trimmed, required, max 255 chars, and cannot contain path separators.
- `parent_folder_id` is optional. `null` or omitted uploads into My Drive root.
  When present it must be an active folder owned by the authenticated user.
- `content_type` is required and must look like `type/subtype`.
- `size_bytes` must be positive, at or below `MAX_FILE_SIZE_BYTES`, and fit inside remaining user quota.
- `checksum_sha256` is optional lowercase SHA-256 hex.

Response `201`:

```json
{
  "file_id": "1f8c6e4d-7752-43e8-b37c-06614a4d0f73",
  "upload_url": "http://localhost:9000/drive-clone/...",
  "object_key": "3f6ad0e9-.../6b5a6c2d-...",
  "expires_at": "2026-07-02T18:15:00Z"
}
```

The client uploads the full object directly to `upload_url` with HTTP `PUT`.
After that, the client must complete the file through the API before it appears
in drive listings.

## Resumable uploads

Resumable uploads use S3-compatible multipart upload state. The API owns
metadata, part records, authorization, and final visibility. The browser uploads
part bytes directly to object storage.

### POST /files/uploads/resumable

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

- Same filename, parent, content type, size, checksum, quota, and max-size rules
  as `POST /files/uploads`.
- `part_size_bytes` is optional. The server defaults to 8 MiB and raises too
  small values when needed to satisfy the S3 multipart minimum part size and
  stay within the S3 multipart limit of 10,000 parts.
- The upload expires at `expires_at`; expired pending sessions cannot sign parts
  or finalize. The session lifetime is `RESUMABLE_UPLOAD_TTL_SECONDS` (default
  86400, i.e. 24h) and is independent of `PRESIGNED_URL_TTL_SECONDS`, which only
  governs how long each individual part upload URL is valid.

Response `201`:

```json
{
  "file_id": "uuid",
  "object_key": "owner/object",
  "part_size_bytes": 8388608,
  "expires_at": "2026-07-06T19:30:00Z"
}
```

### GET /files/uploads/{file_id}/status

Owner-only. Returns the resumable session and confirmed parts:

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

### GET /files/uploads/pending

Authenticated. Lists the caller's own resumable sessions that are still
`pending` and not yet expired (`upload_expires_at > now`). Used by the web client
to recover interrupted uploads across reloads and devices. The server is the
source of truth for which sessions still exist and when they expire.

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

- `parts_received` is the count of recorded `upload_parts` for the session.
- Sessions are ordered by creation time, newest first.
- Finalized, expired, or non-resumable uploads never appear.

### POST /files/uploads/{file_id}/parts

Owner-only. Signs one missing part.

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

The client uploads the exact byte range directly to `upload_url` with HTTP
`PUT`. Object-storage CORS must expose the `ETag` response header.

### POST /files/uploads/{file_id}/parts/{part_number}

Owner-only. Records the uploaded part after direct object-storage PUT succeeds.

Request:

```json
{ "size_bytes": 6291456, "etag": "\"etag-1\"" }
```

Rules:

- `size_bytes` must equal the server's expected size for that part.
- Recording the same part number again replaces the previous ETag and size.

Response `200`:

```json
{ "part_number": 1, "size_bytes": 6291456, "etag": "\"etag-1\"" }
```

### POST /files/uploads/{file_id}/finalize

Owner-only. Requires every expected part to be recorded in order and with the
expected size. The API completes the multipart upload in object storage, HEADs
the final object, marks the file `complete`, and increments storage usage once.

Response `200`: `FileResponse`.

### POST /files/uploads/cleanup-expired

Authenticated. Expires up to 100 stale pending resumable uploads and aborts
their multipart uploads in object storage.

Response `200`:

```json
{ "expired_count": 2, "aborted_count": 2 }
```

## POST /files/{file_id}/complete

The API verifies the authenticated user owns the pending file, HEADs the object in storage, and requires the object content length to match the expected size. On success it marks the file complete and increments `users.storage_used_bytes` exactly once.

Response `200`:

```json
{
  "id": "1f8c6e4d-7752-43e8-b37c-06614a4d0f73",
  "filename": "report.pdf",
  "parent_folder_id": null,
  "content_type": "application/pdf",
  "size_bytes": 12345,
  "checksum_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "object_key": "3f6ad0e9-.../6b5a6c2d-...",
  "state": "complete",
  "created_at": "2026-07-02T18:00:00Z",
  "updated_at": "2026-07-02T18:01:00Z",
  "completed_at": "2026-07-02T18:01:00Z"
}
```

## GET /files

Returns completed files owned by the authenticated user, newest first.

Response `200`:

```json
{
  "files": [
    {
      "id": "1f8c6e4d-7752-43e8-b37c-06614a4d0f73",
      "filename": "report.pdf",
      "parent_folder_id": null,
      "content_type": "application/pdf",
      "size_bytes": 12345,
      "checksum_sha256": null,
      "object_key": "3f6ad0e9-.../6b5a6c2d-...",
      "state": "complete",
      "created_at": "2026-07-02T18:00:00Z",
      "updated_at": "2026-07-02T18:01:00Z",
      "completed_at": "2026-07-02T18:01:00Z"
    }
  ]
}
```

## GET /search?q=<query>&include_deleted=false&limit=20

Searches completed files by filename. Results are scoped to files the
authenticated user can access: owned files plus files shared with the user.
Private files owned by other users never appear.

Query parameters:

- `q`: required after trimming, 1-100 characters.
- `include_deleted`: optional, default `false`. When `true`, deleted owned
  files can appear. Deleted shared files still do not appear because deleting a
  file suspends shared access.
- `limit`: optional, default `20`, clamped to `1..50`.

Matching is case-insensitive substring search on `filename`. Literal `%`, `_`,
and `\` characters are treated as normal query text, not SQL wildcards.

Ordering:

1. Exact filename match.
2. Filename prefix match.
3. Filename substring match.
4. `completed_at DESC`.
5. `id DESC`.

Response `200`:

```json
{
  "query": "report",
  "files": [
    {
      "access": "owned",
      "file": {
        "id": "...",
        "filename": "report.pdf",
        "parent_folder_id": null,
        "content_type": "application/pdf",
        "size_bytes": 12345,
        "checksum_sha256": null,
        "object_key": "...",
        "state": "complete",
        "created_at": "...",
        "updated_at": "...",
        "completed_at": "...",
        "deleted_at": null
      },
      "owner": null
    },
    {
      "access": "shared",
      "file": {
        "id": "...",
        "filename": "shared-report.pdf",
        "parent_folder_id": null,
        "content_type": "application/pdf",
        "size_bytes": 12345,
        "checksum_sha256": null,
        "object_key": "...",
        "state": "complete",
        "created_at": "...",
        "updated_at": "...",
        "completed_at": "...",
        "deleted_at": null
      },
      "owner": { "id": "...", "email": "owner@example.com", "display_name": "Owner" }
    }
  ]
}
```

## Folders, browse, rename, and move

Folder names use the same validation as filenames: trimmed, required, max 255
chars, no path separators, and not `.` or `..`. Duplicate names in the same
folder are allowed.

`FolderResponse`:

```json
{
  "id": "7fd1e2de-cbc1-4f29-9d9f-3aaf8fb5366e",
  "name": "Projects",
  "parent_folder_id": null,
  "created_at": "2026-07-06T12:00:00Z",
  "updated_at": "2026-07-06T12:00:00Z",
  "deleted_at": null
}
```

### POST /folders

Creates a folder in My Drive root or inside another active owned folder.

Request:

```json
{ "name": "Projects", "parent_folder_id": null }
```

Response `201`: `FolderResponse`.

### GET /drive?parent_folder_id=<uuid>

Browses My Drive root when the query param is omitted. When `parent_folder_id`
is present, the folder must be active and owned by the authenticated user.
Returns active child folders and completed active files only.

Response `200`:

```json
{
  "parent_folder_id": null,
  "breadcrumbs": [{ "id": "...", "name": "Projects" }],
  "folders": [],
  "files": []
}
```

### GET /folders

Returns all active folders owned by the user as a flat list for move dialogs:

```json
{ "folders": [ { "id": "...", "name": "Projects", "parent_folder_id": null, "created_at": "...", "updated_at": "...", "deleted_at": null } ] }
```

### PATCH /files/{file_id}

Owner-only rename and/or move. File must be `complete` and not deleted. Moving
to `null` places the file in My Drive root.

Request:

```json
{ "filename": "renamed.pdf", "parent_folder_id": null }
```

Response `200`: `FileResponse`.

### PATCH /folders/{folder_id}

Owner-only rename and/or move. Folder must not be deleted. Moving a folder into
itself or any descendant returns `409 invalid_file_state`.

Request:

```json
{ "name": "Renamed", "parent_folder_id": null }
```

Response `200`: `FolderResponse`.

## GET /files/{file_id}/download

Allowed when the authenticated user is the owner **or** a share grantee. The file must be `complete` and not deleted (`deleted_at` null). Returns a short-lived presigned GET URL.

Response `200`:

```json
{
  "download_url": "http://localhost:9000/drive-clone/...",
  "expires_at": "2026-07-02T18:15:00Z"
}
```

## Soft delete and trash

`FileResponse` gains a `deleted_at` field (`string | null`). `GET /files` returns only files with `deleted_at` null.

### DELETE /files/{file_id}

Owner-only. File must be `complete` and not already deleted, otherwise `file_not_found`. Sets `deleted_at = now()`. Storage quota keeps counting the file while it sits in trash. Shares are kept but stop granting access while the file is deleted.

Response: `204 No Content`.

### POST /files/{file_id}/restore

Owner-only. File must be deleted, otherwise `file_not_found`. Clears `deleted_at`.

Response `200`: the full `FileResponse`.

### GET /files/trash

Returns the authenticated user's deleted files, most recently deleted first. Same shape as `GET /files` (each file has a non-null `deleted_at`).

### DELETE /folders/{folder_id}

Owner-only. Recursively soft-deletes the folder, active descendant folders, and
active descendant files. Items already in trash remain independently deleted.

Response: `204 No Content`.

### POST /folders/{folder_id}/restore

Owner-only. Restores the folder tree deleted by that folder delete. Restore
fails with `409 invalid_file_state` when the folder's parent is still deleted.

Response `200`: `FolderResponse`.

### GET /drive/trash

Returns top-level trash entries for the authenticated user's drive:

```json
{
  "parent_folder_id": null,
  "breadcrumbs": [],
  "folders": [],
  "files": []
}
```

## Sharing

File-level, view/download permission only. Private by default: a file is
accessible only to its owner until the owner grants access to another registered
user by email. Revocable public or tokenized share links are a future extension,
not part of this contract.

### POST /files/{file_id}/shares

Owner-only. File must be `complete` and not deleted.

Request:

```json
{ "email": "friend@example.com" }
```

Rules:

- Unknown email → `404 user_not_found`.
- Sharing with yourself → `422 validation_error`.
- Sharing twice with the same user is idempotent (returns the existing share).

Response `201`:

```json
{
  "file_id": "1f8c6e4d-7752-43e8-b37c-06614a4d0f73",
  "grantee": { "id": "9a...", "email": "friend@example.com", "display_name": "Friend" },
  "created_at": "2026-07-03T12:00:00Z"
}
```

### GET /files/{file_id}/shares

Owner-only. Lists current grantees of the file.

Response `200`:

```json
{ "shares": [ { "file_id": "...", "grantee": { "id": "...", "email": "...", "display_name": "..." }, "created_at": "..." } ] }
```

### DELETE /files/{file_id}/shares/{grantee_id}

Owner-only. Revokes the grantee's access. Unknown share → `404 file_not_found`.

Response: `204 No Content`.

### GET /files/shared-with-me

Files shared with the authenticated user that are `complete` and not deleted, newest share first. Each entry is a `FileResponse` plus the owner:

```json
{
  "files": [
    {
      "id": "...",
      "filename": "report.pdf",
      "content_type": "application/pdf",
      "size_bytes": 12345,
      "checksum_sha256": null,
      "object_key": "...",
      "state": "complete",
      "created_at": "...",
      "completed_at": "...",
      "deleted_at": null,
      "owner": { "id": "...", "email": "owner@example.com", "display_name": "Owner" }
    }
  ]
}
```

## Sync

Cursor-based change feed scoped to the authenticated owner. A client persists a
single integer cursor and polls for everything that changed since. The feed is
gap-free and monotonic per owner: every mutation bumps `users.change_seq` under
a row lock and appends a `change_log` row in the same transaction, so visibility
order always matches sequence order.

Semantics:

- `upsert` covers create, upload completion, rename, move, and restore. The
  entry embeds a snapshot of the current entity to avoid a follow-up fetch.
- `delete` is a tombstone. Trash (soft delete) emits `delete`; a restore later
  emits a new `upsert` with a higher `seq`. For a sync client, an item in the
  trash is an item removed from the drive. Tombstones carry only `entity_id`.
- Trashing a folder emits one `delete` per node in the subtree (every folder and
  every file underneath), each with its own `seq`.
- Pending uploads never appear; only a completed upload emits its first entry.
- The feed is per owner. A user never sees another user's changes, and `seq`
  starts at `1` for each owner independently.

### GET /sync/changes?cursor=\<seq>&limit=\<n>

Authenticated. Returns entries with `seq > cursor`, ascending.

- `cursor` (optional, default `0`): the last `seq` the client has consumed. A
  first-time client sends `0`. Negative → `422 validation_error`.
- `limit` (optional, default `100`): clamped to `1..=500`.

Response `200`:

```json
{
  "changes": [
    {
      "seq": 42,
      "entity_type": "file",
      "op": "upsert",
      "entity_id": "...",
      "occurred_at": "...",
      "file": {
        "id": "...",
        "filename": "report.pdf",
        "parent_folder_id": null,
        "size_bytes": 12345,
        "content_type": "application/pdf",
        "checksum_sha256": null,
        "updated_at": "..."
      }
    },
    {
      "seq": 43,
      "entity_type": "folder",
      "op": "delete",
      "entity_id": "...",
      "occurred_at": "..."
    }
  ],
  "next_cursor": 43,
  "has_more": false
}
```

- `next_cursor` is the `seq` of the last returned entry, or the request cursor
  when the page is empty. Feed it back as `cursor` on the next call.
- `has_more` is `true` when `changes.len() == limit`; keep paging until it is
  `false`.
- `upsert` of a folder embeds a `folder` snapshot (`id`, `name`,
  `parent_folder_id`, `updated_at`) instead of `file`. Tombstones omit both.

## Future Contracts

- Share links: revocable tokenized links separate from the current registered
  user email grants.
