#!/usr/bin/env bash
set -euo pipefail

API_BASE_URL="${API_BASE_URL:-http://localhost:8080}"
WEB_BASE_URL="${WEB_BASE_URL:-http://localhost:3000}"

node - "$API_BASE_URL" "$WEB_BASE_URL" <<'NODE'
const { chromium } = require(process.cwd() + "/apps/web/node_modules/playwright");
const fs = require("fs");
const os = require("os");
const path = require("path");

const apiBaseUrl = process.argv[2];
const webBaseUrl = process.argv[3];
const stamp = `${Date.now()}-${Math.random().toString(16).slice(2)}`;
const filename = "ChatGPT Image 2 de jul. de 2026, 17_22_45.png";
const sizeBytes = 932800;
const lastModified = 1720000000000;
const contentType = "image/png";
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

function writeFixture() {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "drive-resume-ui-"));
  const filePath = path.join(directory, filename);
  const buffer = Buffer.alloc(sizeBytes);
  for (let index = 0; index < sizeBytes; index += 1) {
    buffer[index] = index % 251;
  }
  fs.writeFileSync(filePath, buffer);
  const modified = new Date(lastModified + 120000);
  fs.utimesSync(filePath, modified, modified);
  return filePath;
}

(async () => {
  const filePath = writeFixture();
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

  const chooserPromise = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Resume" }).click();
  const chooser = await chooserPromise;
  await chooser.setFiles(filePath);

  await page
    .getByText("File action failed")
    .waitFor({ timeout: 3000 })
    .then(async () => {
      throw new Error(await page.locator("body").innerText());
    })
    .catch((error) => {
      if (!String(error.message).includes("Timeout")) {
        throw error;
      }
    });
  await page.waitForFunction(
    () => !document.body.innerText.includes("1 upload can resume"),
    null,
    { timeout: 30000 },
  );

  const status = await api(`/files/uploads/${upload.file_id}/status`, {
    token: auth.token,
  });
  if (status.state !== "complete") {
    throw new Error(`expected original upload to complete, got ${status.state}`);
  }
  await browser.close();
  console.log(`resumable resume UI smoke passed against ${webBaseUrl} (${upload.file_id})`);
})().catch(async (error) => {
  console.error(error);
  process.exit(1);
});
NODE
