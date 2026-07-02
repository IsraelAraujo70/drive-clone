# Auth Contract

Base URL: the API service root (local: `http://localhost:8080`, production: the Railway API domain).

All request and response bodies are JSON. Authenticated endpoints require the header:

```
Authorization: Bearer <token>
```

## Token semantics

- Tokens are opaque: 32 random bytes, base64url without padding (43 characters).
- The server stores only the SHA-256 hash of the token. A token is shown once and cannot be recovered.
- Sessions expire 30 days after creation. Expired sessions behave exactly like invalid tokens (401).
- Logout deletes the session server-side; the token stops working immediately.

## User object

```json
{
  "id": "3f6ad0e9-…",
  "email": "user@example.com",
  "display_name": "User Name",
  "storage_quota_bytes": 16106127360,
  "storage_used_bytes": 0,
  "created_at": "2026-07-02T18:00:00Z"
}
```

`password_hash` is never returned.

## Error shape

Every error response has:

```json
{ "error": "<machine_code>", "message": "<human readable, safe to show in UI>" }
```

## Endpoints

### POST /auth/signup

Request: `{ "email": string, "password": string, "display_name": string }`

- Email is trimmed and lowercased before storing; uniqueness is case-insensitive as a result.
- Password: 8 to 128 characters. Display name: 1 to 100 characters after trim.

Responses:

- `201` → `{ "user": User, "token": string }` (a session is created on signup)
- `422 validation_error` → invalid email, password length, or display name
- `409 email_taken` → an account with this email already exists

### POST /auth/login

Request: `{ "email": string, "password": string }`

Responses:

- `200` → `{ "user": User, "token": string }`
- `401 invalid_credentials` → wrong password or unknown email (deliberately indistinguishable)

### POST /auth/logout (authenticated)

Deletes the current session.

Responses:

- `204` (no body)
- `401 unauthorized` → missing, invalid, or expired token

### GET /auth/me (authenticated)

Responses:

- `200` → `User`
- `401 unauthorized` → missing, invalid, or expired token

### GET /health

- `200` → `{ "status": "ok", "service": "drive-clone-api" }` when the database is reachable
- `503` → `{ "status": "degraded", … }` otherwise
