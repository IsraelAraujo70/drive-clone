# Drive Clone Web

Next.js (App Router) frontend for the Drive Clone, styled with Tailwind CSS and shadcn/ui.

## Theme

Public pages (`/`, `/login`, and `/signup`) follow the user's system color
scheme and do not expose a theme toggle. The authenticated drive (`/drive`) has
a header theme button that persists the user's explicit light/dark preference in
`localStorage` under `drive_clone_app_theme`.

## Develop

```bash
npm install
npm run dev   # http://localhost:3000
```

Set `NEXT_PUBLIC_API_BASE_URL` (see `.env.example`) to point at the API. Defaults: local API in development, the Railway API in production builds.

## Test and build

```bash
npm test        # vitest (lib units)
npm run typecheck
npm run lint
npm run eval:theme
npm run build   # standalone output, served by Dockerfile with node server.js
```
