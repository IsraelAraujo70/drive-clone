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
- Verified deployment: `86164b07-f605-4724-9efb-32e3464baf9c`

Frontend deploy:

- Service: `web`
- Service ID: `70b2c191-e986-49bf-b8aa-f21681812a6f`
- Source directory: `apps/web`
- Health endpoint: `/`
- Public URL: `https://web-production-c3311.up.railway.app`
- Verified deployment: `ef2f4843-f122-400b-92ff-ec0157329d67`

Source configuration:

- Repo: `IsraelAraujo70/drive-clone`
- Branch: `google-drive-clone-challenge`
- API root directory: `/services/api`
- Web root directory: `/apps/web`
- API watch pattern: `services/api/**`
- Web watch pattern: `apps/web/**`

Smoke check:

```bash
curl https://api-production-bcad4.up.railway.app/health
curl -I https://web-production-c3311.up.railway.app/
```

Worker deploy:

- Service: `worker`
- Source directory: `services/api`
- Start command: `/usr/local/bin/drive-clone-worker`
- Required env vars: same `DATABASE_URL` and S3 variables as `api`
- Worker-only env vars: `TRASH_RETENTION_DAYS=30`,
  `WORKER_INTERVAL_SECONDS=300`
- No public domain or health endpoint is required. Use logs for `job complete`
  and `job failed`.

Future resources:

- PostgreSQL for metadata.
- S3-compatible bucket for file bytes.
