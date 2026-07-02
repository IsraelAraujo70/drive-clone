# Drive Clone Web

Next.js (App Router) frontend for the Drive Clone, styled with Tailwind CSS and shadcn/ui.

## Develop

```bash
npm install
npm run dev   # http://localhost:3000
```

Set `NEXT_PUBLIC_API_BASE_URL` (see `.env.example`) to point at the API. Defaults: local API in development, the Railway API in production builds.

## Test and build

```bash
npm test        # vitest (lib units)
npm run build   # standalone output, served by Dockerfile with node server.js
```
