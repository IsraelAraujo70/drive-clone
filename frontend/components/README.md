# Front-End Component Structure

This front-end uses atomic design inside domain boundaries.

`components/ui/` is vendored shadcn source. Do not move it into the atomic tree,
and do not wrap it just to rename it. Product components compose these
primitives from domain folders.

Use the levels this way:

- `atoms/`: tiny reusable UI atoms with no product workflow ownership.
- `molecules/`: small product UI chunks that receive state and callbacks.
- `organisms/`: larger composed surfaces such as headers, sidebars, cards, and dialogs.
- `templates/`: route-level page composition. Route files should import templates, not deep molecules.

Pure deterministic logic belongs in `frontend/lib/` so Vitest can test it
without rendering React. Domain hooks can live beside the domain components when
they coordinate browser state, refs, or API calls.

Keep product component files under 350 lines. Templates should stay under 200
lines where practical. Run `make eval-frontend-components` before shipping a
front-end structure change.
