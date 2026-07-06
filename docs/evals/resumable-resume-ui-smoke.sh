#!/usr/bin/env bash
set -euo pipefail

API_BASE_URL="${API_BASE_URL:-http://localhost:8080}"
WEB_BASE_URL="${WEB_BASE_URL:-http://localhost:3000}"

node - "$API_BASE_URL" "$WEB_BASE_URL" <<'NODE'
const { chromium } = require(process.cwd() + "/apps/web/node_modules/playwright");

const apiBaseUrl = process.argv[2];
const webBaseUrl = process.argv[3];
const stamp = `${Date.now()}-${Math.random().toString(16).slice(2)}`;
const filename = "BKEEPER.dmg";
const sizeBytes = 6 * 1024 * 1024 + 37;
const lastModified = 1720000000000;
const contentType = "application/x-apple-diskimage";
const storageKey = [
  filename,
  sizeBytes,
  lastModified,
  contentType,
  "root",
].join(":");

async function api(path, options = {}) {
  const response = await fetch(`${apiBaseUrl}${path}`, {
    ...options,
    headers: {
      ...(options.body ? { "content-type": "application/json" } : {}),
      ...(options.token ? { authorization: `Bearer ${options.token}` } : {}),
      ...(options.headers ?? {}),
    },
  });
  const text = await response.text();
  const body = text ? JSON.parse(text) : null;
  if (!response.ok) {
    throw new Error(`${path} failed ${response.status}: ${text}`);
  }
  return body;
}

(async () => {
  const auth = await api("/auth/signup", {
    method: "POST",
    body: JSON.stringify({
      email: `resume-ui-${stamp}@example.com`,
      password: "password123",
      display_name: "Resume UI Smoke",
    }),
  });
  const upload = await api("/files/uploads/resumable", {
    method: "POST",
    token: auth.token,
    body: JSON.stringify({
      filename,
      parent_folder_id: null,
      content_type: contentType,
      size_bytes: sizeBytes,
      checksum_sha256: null,
      part_size_bytes: 6 * 1024 * 1024,
    }),
  });
  const pending = {
    [storageKey]: {
      file_id: upload.file_id,
      filename,
      size_bytes: sizeBytes,
      last_modified: lastModified,
      content_type: contentType,
      parent_folder_id: null,
      expires_at: upload.expires_at,
    },
  };

  const browser = await chromium.launch();
  const page = await browser.newPage();
  await page.addInitScript(
    ({ token, pending }) => {
      localStorage.setItem("drive_clone_token", token);
      localStorage.setItem(
        "drive_clone_resumable_uploads_v1",
        JSON.stringify(pending),
      );
    },
    { token: auth.token, pending },
  );
  await page.goto(`${webBaseUrl}/drive`, { waitUntil: "domcontentloaded" });
  await page.getByText("1 upload can resume").waitFor({ timeout: 15000 });
  await page.getByText(filename).waitFor({ timeout: 15000 });
  await page
    .getByText("Select the same local file again to continue from the parts already saved.")
    .waitFor({ timeout: 15000 });
  await browser.close();
  console.log(`resumable resume UI smoke passed against ${webBaseUrl} (${upload.file_id})`);
})().catch(async (error) => {
  console.error(error);
  process.exit(1);
});
NODE

