# API Service

Rust backend for the Google Drive clone.

Current scope:

- Provide a deployable HTTP service.
- Expose `/health` for Railway health checks.
- Authenticate users.
- Create direct-upload sessions, verify completed objects, list completed files, and return authorized download URLs.

Local commands:

```bash
cargo test
cargo run
```

The server reads:

- `HOST`, default `0.0.0.0`
- `PORT`, default `8080`
- `DATABASE_URL`, required
- `S3_ENDPOINT_URL`, required for API runtime
- `S3_PUBLIC_ENDPOINT_URL`, optional browser-facing endpoint used when MinIO is reachable through a different host from the API container
- `S3_BUCKET`, required
- `S3_REGION`, required
- `S3_URL_STYLE`, optional: `path` for MinIO/local S3 or `virtual-host` for Railway buckets
- `S3_ACCESS_KEY_ID`, required
- `S3_SECRET_ACCESS_KEY`, required
- `CORS_ALLOWED_ORIGINS`, comma-separated local web origins, for example `http://localhost:3000,http://127.0.0.1:3000`
- `MAX_FILE_SIZE_BYTES`, default `16106127360`
- `PRESIGNED_URL_TTL_SECONDS`, default `900`

Local MinIO example:

```bash
docker compose up -d

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
```

If the web dev server uses another port, add that exact origin to `CORS_ALLOWED_ORIGINS`, for example `http://localhost:3100`.

When the API runs inside Docker and the browser runs on the host, sign URLs with `S3_PUBLIC_ENDPOINT_URL=http://localhost:9000` while using `S3_ENDPOINT_URL=http://minio:9000` for server-side object HEAD checks. The signed URL host must match the host the browser sends to MinIO. Railway buckets use `S3_URL_STYLE=virtual-host`.
