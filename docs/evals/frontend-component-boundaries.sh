#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

MAX_LINES=350
failed=0

printf 'Largest frontend component files:\n'
find frontend/components \
  -path frontend/components/ui -prune -o \
  -type f -name '*.tsx' -print0 |
  xargs -0 wc -l |
  sort -nr |
  head -20

while IFS= read -r -d '' file; do
  lines="$(wc -l < "$file" | tr -d ' ')"
  if (( lines > MAX_LINES )); then
    printf 'FAIL: %s has %s lines; limit is %s\n' "$file" "$lines" "$MAX_LINES" >&2
    failed=1
  fi
done < <(
  find frontend/components \
    -path frontend/components/ui -prune -o \
    -type f -name '*.tsx' -print0
)

while IFS= read -r -d '' file; do
  printf 'FAIL: root-level product component remains: %s\n' "$file" >&2
  failed=1
done < <(find frontend/components -maxdepth 1 -type f -name '*.tsx' -print0)

old_paths=(
  frontend/components/drive-shell.tsx
  frontend/components/landing.tsx
  frontend/components/login-form.tsx
  frontend/components/signup-form.tsx
  frontend/components/reset-password-form.tsx
  frontend/components/password-input.tsx
  frontend/components/app-sidebar.tsx
  frontend/components/command-menu.tsx
)

for path in "${old_paths[@]}"; do
  if [[ -e "$path" ]]; then
    printf 'FAIL: old component path still exists: %s\n' "$path" >&2
    failed=1
  fi
done

old_imports='@/components/(drive-shell|app-sidebar|command-menu|landing|login-form|signup-form|reset-password-form|password-input|brand|theme-provider)(["'\'']|$)'
if rg -n "$old_imports" frontend/app frontend/components --glob '*.tsx' --glob '*.ts'; then
  printf 'FAIL: old root component import found\n' >&2
  failed=1
fi

if (( failed != 0 )); then
  exit 1
fi

printf 'PASS: frontend component boundaries are within the atomic domain structure.\n'
