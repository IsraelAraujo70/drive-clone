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
- upload input validation
- quota calculation
- domain error categories

This layer must not import Axum, SQLx, Reqwest, environment variables, or object-storage adapters.

### Application

`application` owns use cases and ports:

- auth use cases: signup, login, logout, current user lookup
- file use cases: create upload, complete upload, list files, download file
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
2. The browser uploads bytes directly to S3-compatible object storage.
3. `POST /files/{file_id}/complete` checks ownership, verifies object length with storage, marks the row complete, and increments storage usage once.
4. `GET /files` lists completed files owned by the user.
5. `GET /files/{file_id}/download` checks ownership and returns a short-lived signed GET URL.
