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

Verifies ownership and complete state, then returns a short-lived presigned GET URL.

Response `200`:

```json
{
  "download_url": "http://localhost:9000/drive-clone/...",
  "expires_at": "2026-07-02T18:15:00Z"
}
```
