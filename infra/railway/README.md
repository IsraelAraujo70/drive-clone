# Railway Infrastructure

Railway is the preferred deployment target.

Initial deploy:

- Project: `drive-clone`
- Project ID: `ae72dbc0-fd9b-405c-928d-accc0b21a8bd`
- Service: `api`
- Service ID: `967c1642-07c7-4c92-8b7a-a1203d9d24a3`
- Source directory: `services/api`
- Health endpoint: `/health`
- Public URL: `https://api-production-bcad4.up.railway.app`
- Verified deployment: `51137aad-427c-449f-b231-52f2d34681b7`

Smoke check:

```bash
curl https://api-production-bcad4.up.railway.app/health
```

Future resources:

- `web` service for the TypeScript frontend.
- PostgreSQL for metadata.
- S3-compatible bucket for file bytes.
- Optional worker service for cleanup jobs.
