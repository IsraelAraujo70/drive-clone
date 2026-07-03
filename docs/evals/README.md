# Evals

## MinIO Upload Smoke

Run after the API is serving against docker-compose Postgres and MinIO:

```bash
docker compose up -d
cd services/api
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

Then in another terminal:

```bash
bash docs/evals/minio-upload-smoke.sh
```

The script signs up a user, creates a direct upload, PUTs bytes to MinIO, completes the file, verifies listing, requests a download URL, and byte-compares the downloaded object.
