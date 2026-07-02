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
- Verified deployment: `91e7da03-f664-4782-948f-b78ebee06659`

Frontend deploy:

- Service: `web`
- Service ID: `70b2c191-e986-49bf-b8aa-f21681812a6f`
- Source directory: `apps/web`
- Health endpoint: `/`
- Public URL: `https://web-production-c3311.up.railway.app`
- Verified deployment: `2c3f6d98-18bf-4d69-8259-57e462aea352`

Source configuration:

- Repo: `IsraelAraujo70/drive-clone`
- Branch: `google-drive-clone-challenge`
- API root directory: `/services/api`
- Web root directory: `/apps/web`

Smoke check:

```bash
curl https://api-production-bcad4.up.railway.app/health
curl -I https://web-production-c3311.up.railway.app/
```

Future resources:

- `web` service for the TypeScript frontend.
- PostgreSQL for metadata.
- S3-compatible bucket for file bytes.
- Optional worker service for cleanup jobs.
