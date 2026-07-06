#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const outDir = path.join(root, "docs/api/bruno");

const collection = {
  version: "1",
  name: "Drive Clone API",
  type: "collection",
  ignore: ["node_modules", ".git"],
};

const environments = [
  {
    name: "Development",
    vars: {
      baseUrl: "http://localhost:8080",
      authToken: "",
      userId: "",
      fileId: "",
      folderId: "",
      granteeId: "",
      granteeEmail: "friend@example.com",
      uploadUrl: "",
      partUploadUrl: "",
      objectKey: "",
      partNumber: "1",
      partSizeBytes: "6291456",
      searchQuery: "report",
      signupEmail: "bruno-{{$timestamp}}@example.com",
      signupPassword: "password123",
      signupDisplayName: "Bruno User",
      loginEmail: "user@example.com",
      loginPassword: "password123",
      checksumSha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      sizeBytes: "12",
      syncCursor: "0",
      syncLimit: "100",
    },
  },
  {
    name: "Production",
    vars: {
      baseUrl: "https://api-production-bcad4.up.railway.app",
      authToken: "",
      userId: "",
      fileId: "",
      folderId: "",
      granteeId: "",
      granteeEmail: "friend@example.com",
      uploadUrl: "",
      partUploadUrl: "",
      objectKey: "",
      partNumber: "1",
      partSizeBytes: "6291456",
      searchQuery: "report",
      signupEmail: "bruno-{{$timestamp}}@example.com",
      signupPassword: "password123",
      signupDisplayName: "Bruno User",
      loginEmail: "user@example.com",
      loginPassword: "password123",
      checksumSha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      sizeBytes: "12",
      syncCursor: "0",
      syncLimit: "100",
    },
  },
];

const folders = [
  {
    dir: "system",
    name: "System",
    requests: [
      {
        name: "Root",
        method: "get",
        url: "{{baseUrl}}/",
        auth: "none",
        docs: "Returns the API service identity and product message.",
        tests: ["expect(res.status).to.equal(200);"],
      },
      {
        name: "Health",
        method: "get",
        url: "{{baseUrl}}/health",
        auth: "none",
        docs: "Checks database reachability. Healthy responses return status ok and service drive-clone-api.",
        tests: ["expect([200, 503]).to.include(res.status);"],
      },
    ],
  },
  {
    dir: "auth",
    name: "Auth",
    requests: [
      {
        name: "Signup",
        method: "post",
        url: "{{baseUrl}}/auth/signup",
        auth: "none",
        body: {
          email: "{{signupEmail}}",
          password: "{{signupPassword}}",
          display_name: "{{signupDisplayName}}",
        },
        docs: "Creates an account and session. Email is trimmed/lowercased; password must be 8-128 chars. Saves token and user id to the active environment.",
        postResponse: [
          "if (res.body && res.body.token) {",
          "  bru.setVar('authToken', res.body.token);",
          "}",
          "if (res.body && res.body.user && res.body.user.id) {",
          "  bru.setVar('userId', res.body.user.id);",
          "}",
        ],
        tests: [
          "expect(res.status).to.equal(201);",
          "expect(res.body).to.have.property('token');",
        ],
      },
      {
        name: "Login",
        method: "post",
        url: "{{baseUrl}}/auth/login",
        auth: "none",
        body: {
          email: "{{loginEmail}}",
          password: "{{loginPassword}}",
        },
        docs: "Authenticates an existing user. Unknown email and wrong password both return invalid_credentials. Saves token and user id to the active environment.",
        postResponse: [
          "if (res.body && res.body.token) {",
          "  bru.setVar('authToken', res.body.token);",
          "}",
          "if (res.body && res.body.user && res.body.user.id) {",
          "  bru.setVar('userId', res.body.user.id);",
          "}",
        ],
        tests: ["expect([200, 401]).to.include(res.status);"],
      },
      {
        name: "Me",
        method: "get",
        url: "{{baseUrl}}/auth/me",
        auth: "bearer",
        docs: "Returns the current authenticated user. Tokens are opaque and stored server-side as SHA-256 hashes.",
        tests: ["expect([200, 401]).to.include(res.status);"],
      },
      {
        name: "Logout",
        method: "post",
        url: "{{baseUrl}}/auth/logout",
        auth: "bearer",
        docs: "Deletes the current server-side session. The current bearer token stops working immediately.",
        tests: ["expect([204, 401]).to.include(res.status);"],
      },
    ],
  },
  {
    dir: "drive",
    name: "Drive",
    requests: [
      {
        name: "Browse Root",
        method: "get",
        url: "{{baseUrl}}/drive",
        auth: "bearer",
        docs: "Browses My Drive root. Returns active child folders and completed active files only.",
        tests: ["expect([200, 401]).to.include(res.status);"],
      },
      {
        name: "Browse Folder",
        method: "get",
        url: "{{baseUrl}}/drive",
        auth: "bearer",
        query: { parent_folder_id: "{{folderId}}" },
        docs: "Browses an active folder owned by the authenticated user. Private or missing folders return file_not_found.",
        tests: ["expect([200, 401, 404]).to.include(res.status);"],
      },
      {
        name: "Search Files",
        method: "get",
        url: "{{baseUrl}}/search",
        auth: "bearer",
        query: {
          q: "{{searchQuery}}",
          include_deleted: "false",
          limit: "20",
        },
        docs: "Searches completed files by filename. Scope is owned plus shared-with-me. Trash is excluded by default and private files never appear.",
        tests: ["expect([200, 401, 422]).to.include(res.status);"],
      },
      {
        name: "Search Files Including Own Trash",
        method: "get",
        url: "{{baseUrl}}/search",
        auth: "bearer",
        query: {
          q: "{{searchQuery}}",
          include_deleted: "true",
          limit: "20",
        },
        docs: "Searches active accessible files plus deleted files owned by the authenticated user. Deleted shared files stay hidden.",
        tests: ["expect([200, 401, 422]).to.include(res.status);"],
      },
      {
        name: "Drive Trash",
        method: "get",
        url: "{{baseUrl}}/drive/trash",
        auth: "bearer",
        docs: "Returns top-level trash entries for folders and files owned by the authenticated user.",
        tests: ["expect([200, 401]).to.include(res.status);"],
      },
    ],
  },
  {
    dir: "sync",
    name: "Sync",
    requests: [
      {
        name: "List Sync Changes",
        method: "get",
        url: "{{baseUrl}}/sync/changes",
        auth: "bearer",
        query: {
          cursor: "{{syncCursor}}",
          limit: "{{syncLimit}}",
        },
        docs: "Per-owner cursor-based change feed with tombstones. Send cursor=0 first, then feed next_cursor back. Upserts embed the entity snapshot; deletes carry only entity_id. Negative cursor returns validation_error.",
        tests: ["expect([200, 401, 422]).to.include(res.status);"],
      },
    ],
  },
  {
    dir: "folders",
    name: "Folders",
    requests: [
      {
        name: "Create Folder",
        method: "post",
        url: "{{baseUrl}}/folders",
        auth: "bearer",
        body: {
          name: "Bruno Folder",
          parent_folder_id: null,
        },
        docs: "Creates a folder in My Drive root or inside an active owned folder. Names are trimmed, max 255 chars, and cannot contain path separators.",
        postResponse: [
          "if (res.body && res.body.id) {",
          "  bru.setVar('folderId', res.body.id);",
          "}",
        ],
        tests: ["expect([201, 401, 404, 422]).to.include(res.status);"],
      },
      {
        name: "List Folders",
        method: "get",
        url: "{{baseUrl}}/folders",
        auth: "bearer",
        docs: "Returns all active folders owned by the user as a flat list for move dialogs.",
        tests: ["expect([200, 401]).to.include(res.status);"],
      },
      {
        name: "Update Folder",
        method: "patch",
        url: "{{baseUrl}}/folders/{{folderId}}",
        auth: "bearer",
        body: {
          name: "Bruno Folder Renamed",
          parent_folder_id: null,
        },
        docs: "Owner-only rename and/or move. Moving a folder into itself or a descendant returns invalid_file_state.",
        tests: ["expect([200, 401, 404, 409, 422]).to.include(res.status);"],
      },
      {
        name: "Delete Folder",
        method: "delete",
        url: "{{baseUrl}}/folders/{{folderId}}",
        auth: "bearer",
        docs: "Owner-only recursive soft delete for the folder tree and active descendant files.",
        tests: ["expect([204, 401, 404]).to.include(res.status);"],
      },
      {
        name: "Restore Folder",
        method: "post",
        url: "{{baseUrl}}/folders/{{folderId}}/restore",
        auth: "bearer",
        docs: "Owner-only restore for the folder tree deleted by that folder delete. Fails if the parent remains deleted.",
        tests: ["expect([200, 401, 404, 409]).to.include(res.status);"],
      },
    ],
  },
  {
    dir: "files",
    name: "Files",
    requests: [
      {
        name: "Create Upload",
        method: "post",
        url: "{{baseUrl}}/files/uploads",
        auth: "bearer",
        body: {
          filename: "bruno-report.txt",
          parent_folder_id: null,
          content_type: "text/plain",
          size_bytes: 12,
          checksum_sha256: "{{checksumSha256}}",
        },
        docs: "Creates pending file metadata and returns a short-lived presigned PUT URL. The client uploads bytes directly to upload_url.",
        postResponse: [
          "if (res.body && res.body.file_id) {",
          "  bru.setVar('fileId', res.body.file_id);",
          "}",
          "if (res.body && res.body.upload_url) {",
          "  bru.setVar('uploadUrl', res.body.upload_url);",
          "}",
          "if (res.body && res.body.object_key) {",
          "  bru.setVar('objectKey', res.body.object_key);",
          "}",
        ],
        tests: ["expect([201, 401, 404, 409, 413, 422]).to.include(res.status);"],
      },
      {
        name: "Direct PUT Upload URL",
        method: "put",
        url: "{{uploadUrl}}",
        auth: "none",
        headers: { "Content-Type": "text/plain" },
        textBody: "hello bruno\n",
        docs: "Uploads raw bytes to the presigned object-storage URL returned by Create Upload. This is not authenticated by the API bearer token.",
        tests: ["expect([200, 201, 204, 403]).to.include(res.status);"],
      },
      {
        name: "Complete Upload",
        method: "post",
        url: "{{baseUrl}}/files/{{fileId}}/complete",
        auth: "bearer",
        docs: "Verifies the uploaded object size and atomically marks the pending file complete. Storage usage increments exactly once.",
        tests: ["expect([200, 401, 404, 409, 502]).to.include(res.status);"],
      },
      {
        name: "Create Resumable Upload",
        method: "post",
        url: "{{baseUrl}}/files/uploads/resumable",
        auth: "bearer",
        body: {
          filename: "bruno-resumable.bin",
          parent_folder_id: null,
          content_type: "application/octet-stream",
          size_bytes: "{{sizeBytes}}",
          checksum_sha256: null,
          part_size_bytes: "{{partSizeBytes}}",
        },
        docs: "Creates a resumable multipart upload session. Prefer this flow for new clients. Saves file id and object key to the active environment.",
        postResponse: [
          "if (res.body && res.body.file_id) {",
          "  bru.setVar('fileId', res.body.file_id);",
          "}",
          "if (res.body && res.body.object_key) {",
          "  bru.setVar('objectKey', res.body.object_key);",
          "}",
          "if (res.body && res.body.part_size_bytes) {",
          "  bru.setVar('partSizeBytes', String(res.body.part_size_bytes));",
          "}",
        ],
        tests: ["expect([201, 401, 404, 409, 413, 422, 502]).to.include(res.status);"],
      },
      {
        name: "Get Upload Status",
        method: "get",
        url: "{{baseUrl}}/files/uploads/{{fileId}}/status",
        auth: "bearer",
        docs: "Returns resumable upload state and confirmed parts so clients can resume from server state.",
        tests: ["expect([200, 401, 404]).to.include(res.status);"],
      },
      {
        name: "Sign Upload Part",
        method: "post",
        url: "{{baseUrl}}/files/uploads/{{fileId}}/parts",
        auth: "bearer",
        body: {
          part_number: "{{partNumber}}",
        },
        docs: "Signs one object-storage multipart PUT URL. Upload exactly expected_size_bytes to the returned URL, then record the ETag.",
        postResponse: [
          "if (res.body && res.body.upload_url) {",
          "  bru.setVar('partUploadUrl', res.body.upload_url);",
          "}",
        ],
        tests: ["expect([200, 401, 404, 409, 422, 502]).to.include(res.status);"],
      },
      {
        name: "Direct PUT Upload Part URL",
        method: "put",
        url: "{{partUploadUrl}}",
        auth: "none",
        textBody: "hello bruno part\n",
        docs: "Uploads raw part bytes to the presigned object-storage URL returned by Sign Upload Part. Real resumable runs must upload exactly expected_size_bytes and preserve the ETag response header.",
        tests: ["expect([200, 201, 204, 403]).to.include(res.status);"],
      },
      {
        name: "Record Upload Part",
        method: "post",
        url: "{{baseUrl}}/files/uploads/{{fileId}}/parts/{{partNumber}}",
        auth: "bearer",
        body: {
          size_bytes: "{{partSizeBytes}}",
          etag: "\"replace-with-object-storage-etag\"",
        },
        docs: "Records one uploaded multipart part. The ETag must come from object storage after PUT; recording the same part replaces prior metadata.",
        tests: ["expect([200, 401, 404, 409, 422]).to.include(res.status);"],
      },
      {
        name: "Finalize Resumable Upload",
        method: "post",
        url: "{{baseUrl}}/files/uploads/{{fileId}}/finalize",
        auth: "bearer",
        docs: "Completes the multipart object, verifies final object length, marks the file complete, and increments storage usage once.",
        tests: ["expect([200, 401, 404, 409, 502]).to.include(res.status);"],
      },
      {
        name: "Cleanup Expired Uploads",
        method: "post",
        url: "{{baseUrl}}/files/uploads/cleanup-expired",
        auth: "bearer",
        docs: "Expires stale pending resumable uploads and aborts their multipart uploads in object storage.",
        tests: ["expect([200, 401, 502]).to.include(res.status);"],
      },
      {
        name: "List Files",
        method: "get",
        url: "{{baseUrl}}/files",
        auth: "bearer",
        docs: "Returns completed active files owned by the authenticated user, newest first.",
        tests: ["expect([200, 401]).to.include(res.status);"],
      },
      {
        name: "Download File",
        method: "get",
        url: "{{baseUrl}}/files/{{fileId}}/download",
        auth: "bearer",
        docs: "Returns a short-lived presigned GET URL when the user owns the active complete file or has a current share grant.",
        tests: ["expect([200, 401, 404, 502]).to.include(res.status);"],
      },
      {
        name: "Update File",
        method: "patch",
        url: "{{baseUrl}}/files/{{fileId}}",
        auth: "bearer",
        body: {
          filename: "bruno-report-renamed.txt",
          parent_folder_id: null,
        },
        docs: "Owner-only rename and/or move for complete active files. Use parent_folder_id null to move to root.",
        tests: ["expect([200, 401, 404, 422]).to.include(res.status);"],
      },
      {
        name: "Delete File",
        method: "delete",
        url: "{{baseUrl}}/files/{{fileId}}",
        auth: "bearer",
        docs: "Owner-only soft delete. Shares are retained but stop granting access while the file is deleted.",
        tests: ["expect([204, 401, 404]).to.include(res.status);"],
      },
      {
        name: "Restore File",
        method: "post",
        url: "{{baseUrl}}/files/{{fileId}}/restore",
        auth: "bearer",
        docs: "Owner-only restore for a file directly deleted by the owner. Clears deleted_at when the parent folder is active.",
        tests: ["expect([200, 401, 404]).to.include(res.status);"],
      },
      {
        name: "List File Trash",
        method: "get",
        url: "{{baseUrl}}/files/trash",
        auth: "bearer",
        docs: "Returns deleted files owned by the authenticated user, most recently deleted first.",
        tests: ["expect([200, 401]).to.include(res.status);"],
      },
      {
        name: "List Shared With Me",
        method: "get",
        url: "{{baseUrl}}/files/shared-with-me",
        auth: "bearer",
        docs: "Returns active complete files shared with the authenticated user. Each file includes owner identity.",
        tests: ["expect([200, 401]).to.include(res.status);"],
      },
    ],
  },
  {
    dir: "shares",
    name: "Shares",
    requests: [
      {
        name: "Create Share",
        method: "post",
        url: "{{baseUrl}}/files/{{fileId}}/shares",
        auth: "bearer",
        body: {
          email: "{{granteeEmail}}",
        },
        docs: "Owner-only grant by registered user email. Self-share returns validation_error; unknown email returns user_not_found; duplicate share is idempotent.",
        postResponse: [
          "if (res.body && res.body.grantee && res.body.grantee.id) {",
          "  bru.setVar('granteeId', res.body.grantee.id);",
          "}",
        ],
        tests: ["expect([201, 401, 404, 422]).to.include(res.status);"],
      },
      {
        name: "List Shares",
        method: "get",
        url: "{{baseUrl}}/files/{{fileId}}/shares",
        auth: "bearer",
        docs: "Owner-only list of current grantees for a file.",
        tests: ["expect([200, 401, 404]).to.include(res.status);"],
      },
      {
        name: "Revoke Share",
        method: "delete",
        url: "{{baseUrl}}/files/{{fileId}}/shares/{{granteeId}}",
        auth: "bearer",
        docs: "Owner-only revoke. Unknown share or cross-user revoke returns file_not_found.",
        tests: ["expect([204, 401, 404]).to.include(res.status);"],
      },
    ],
  },
];

function writeFile(filePath, content) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${content.trimEnd()}\n`);
}

function formatVars(vars) {
  const lines = ["vars {"];
  for (const [key, value] of Object.entries(vars)) {
    lines.push(value === "" ? `  ${key}:` : `  ${key}: ${value}`);
  }
  lines.push("}", "", "vars:secret [", "  authToken", "]");
  return lines.join("\n");
}

function formatKeyValueBlock(name, values) {
  if (!values || Object.keys(values).length === 0) {
    return "";
  }
  const lines = [`${name} {`];
  for (const [key, value] of Object.entries(values)) {
    lines.push(`  ${key}: ${value}`);
  }
  lines.push("}", "");
  return lines.join("\n");
}

function formatJsonBody(body) {
  if (!body) {
    return "";
  }
  const json = JSON.stringify(body, null, 2)
    .split("\n")
    .map((line) => `  ${line}`)
    .join("\n");
  return `body:json {\n${json}\n}\n`;
}

function formatTextBody(textBody) {
  if (textBody === undefined) {
    return "";
  }
  return `body:text {\n${textBody}}\n`;
}

function formatTests(assertions, setup = []) {
  if (!assertions?.length && !setup.length) {
    return "";
  }
  const setupBlock = setup.length ? `  ${setup.join("\n  ")}\n\n` : "";
  const assertionsBlock = assertions.length
    ? `  test("response status is documented", function() {\n    ${assertions.join("\n    ")}\n  });\n`
    : "";
  return `tests {\n${setupBlock}${assertionsBlock}}\n`;
}

function formatDocs(text) {
  if (!text) {
    return "";
  }
  return `docs {\n  ${text}\n}\n`;
}

function bodyKind(route) {
  if (route.body) {
    return "json";
  }
  if (route.textBody !== undefined) {
    return "text";
  }
  return "none";
}

function formatRequest(route, seq) {
  const method = route.method.toLowerCase();
  const headers = {
    Accept: "application/json",
    ...(route.body ? { "Content-Type": "application/json" } : {}),
    ...(route.headers ?? {}),
  };
  const auth = route.auth ?? "bearer";

  return [
    "meta {",
    `  name: ${route.name}`,
    "  type: http",
    `  seq: ${seq}`,
    "}",
    "",
    `${method} {`,
    `  url: ${route.url}`,
    `  body: ${bodyKind(route)}`,
    `  auth: ${auth}`,
    "}",
    "",
    auth === "bearer"
      ? "auth:bearer {\n  token: {{authToken}}\n}\n"
      : "",
    formatKeyValueBlock("query", route.query),
    formatKeyValueBlock("headers", headers),
    formatJsonBody(route.body),
    formatTextBody(route.textBody),
    formatTests(route.tests, route.postResponse),
    formatDocs(route.docs),
  ]
    .filter(Boolean)
    .join("\n");
}

function fileName(name) {
  return `${name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "")}.bru`;
}

fs.rmSync(outDir, { recursive: true, force: true });
fs.mkdirSync(outDir, { recursive: true });
writeFile(path.join(outDir, "bruno.json"), JSON.stringify(collection, null, 2));

for (const env of environments) {
  writeFile(
    path.join(outDir, "environments", `${env.name}.bru`),
    formatVars(env.vars),
  );
}

for (const folder of folders) {
  const folderDir = path.join(outDir, folder.dir);
  writeFile(path.join(folderDir, "folder.bru"), `meta {\n  name: ${folder.name}\n}`);
  folder.requests.forEach((route, index) => {
    writeFile(
      path.join(folderDir, fileName(route.name)),
      formatRequest(route, index + 1),
    );
  });
}

console.log(`Generated Bruno collection at ${path.relative(root, outDir)}`);
