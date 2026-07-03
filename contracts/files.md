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

## POST /files/uploads

Request:

```json
{
  "filename": "report.pdf",
  "content_type": "application/pdf",
  "size_bytes": 12345,
  "checksum_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}
```

Rules:

- `filename` is trimmed, required, max 255 chars, and cannot contain path separators.
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

The client uploads bytes directly to `upload_url` with HTTP `PUT`.

## POST /files/{file_id}/complete

The API verifies the authenticated user owns the pending file, HEADs the object in storage, and requires the object content length to match the expected size. On success it marks the file complete and increments `users.storage_used_bytes` exactly once.

Response `200`:

```json
{
  "id": "1f8c6e4d-7752-43e8-b37c-06614a4d0f73",
  "filename": "report.pdf",
  "content_type": "application/pdf",
  "size_bytes": 12345,
  "checksum_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "object_key": "3f6ad0e9-.../6b5a6c2d-...",
  "state": "complete",
  "created_at": "2026-07-02T18:00:00Z",
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
      "content_type": "application/pdf",
      "size_bytes": 12345,
      "checksum_sha256": null,
      "object_key": "3f6ad0e9-.../6b5a6c2d-...",
      "state": "complete",
      "created_at": "2026-07-02T18:00:00Z",
      "completed_at": "2026-07-02T18:01:00Z"
    }
  ]
}
```

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

## Sharing

File-level, view/download permission only. Private by default: a file is accessible only to its owner until shared.

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
