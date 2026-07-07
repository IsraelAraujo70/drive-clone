type ApiOptions = {
  token?: string
  body?: unknown
  qs?: Record<string, string | number | boolean | null | undefined>
  failOnStatusCode?: boolean
}

type AuthResponse = {
  token: string
  user: {
    id: string
    email: string
    display_name: string
    storage_quota_bytes: number
    storage_used_bytes: number
    created_at: string
  }
}

type CreateUploadResponse = {
  file_id: string
  upload_url: string
  object_key: string
  expires_at: string
}

type CreateResumableUploadResponse = {
  file_id: string
  object_key: string
  part_size_bytes: number
  expires_at: string
}

type UploadBytes = string | Uint8Array | Buffer

type SeedCompletedFileInput = {
  filename: string
  contentType: string
  bytes: UploadBytes
  parentFolderId?: string | null
}

type CreateResumableSessionInput = {
  filename: string
  contentType: string
  sizeBytes: number
  partSizeBytes?: number
  parentFolderId?: string | null
}

function apiBaseUrl() {
  return String(Cypress.env("apiBaseUrl") ?? "http://localhost:8080").replace(
    /\/$/,
    ""
  )
}

function toBuffer(bytes: UploadBytes) {
  if (Cypress.Buffer.isBuffer(bytes)) {
    return bytes
  }
  if (typeof bytes === "string") {
    return Cypress.Buffer.from(bytes)
  }
  return Cypress.Buffer.from(Array.from(bytes))
}

Cypress.Commands.add("api", (method: string, path: string, options = {}) => {
  const typedOptions = options as ApiOptions
  const headers: Record<string, string> = {}
  if (typedOptions.token) {
    headers.Authorization = `Bearer ${typedOptions.token}`
  }

  return cy.request({
    method,
    url: `${apiBaseUrl()}${path}`,
    headers,
    body: typedOptions.body as never,
    qs: typedOptions.qs,
    failOnStatusCode: typedOptions.failOnStatusCode,
  })
})

Cypress.Commands.add(
  "signupByApi",
  (email: string, password = "Password123!") => {
    return cy
      .api<AuthResponse>("POST", "/auth/signup", {
        body: {
          email,
          password,
          display_name: email.split("@")[0],
        },
      })
      .then((response) => response.body)
  }
)

Cypress.Commands.add(
  "loginByApi",
  (email: string, password = "Password123!") => {
    return cy
      .api<AuthResponse>("POST", "/auth/login", {
        body: { email, password },
      })
      .then((response) => response.body)
  }
)

Cypress.Commands.add("authenticatedVisit", (path: string, token: string) => {
  return cy.visit(path, {
    onBeforeLoad(win) {
      win.localStorage.setItem("drive_clone_token", token)
    },
  })
})

Cypress.Commands.add(
  "seedCompletedFile",
  (token: string, input: SeedCompletedFileInput) => {
    const payload = toBuffer(input.bytes)

    return cy
      .api<CreateUploadResponse>("POST", "/files/uploads", {
        token,
        body: {
          filename: input.filename,
          parent_folder_id: input.parentFolderId ?? null,
          content_type: input.contentType,
          size_bytes: payload.length,
          checksum_sha256: null,
        },
      })
      .then((created) => {
        const upload = created.body
        return cy
          .request({
            method: "PUT",
            url: upload.upload_url,
            headers: {
              "Content-Type": input.contentType,
            },
            body: payload.toString("binary"),
            encoding: "binary",
          })
          .then(() =>
            cy
              .api("POST", `/files/${upload.file_id}/complete`, { token })
              .then(() => ({
                fileId: upload.file_id,
                objectKey: upload.object_key,
              }))
          )
      })
  }
)

Cypress.Commands.add(
  "createResumableSession",
  (token: string, input: CreateResumableSessionInput) => {
    return cy
      .api<CreateResumableUploadResponse>("POST", "/files/uploads/resumable", {
        token,
        body: {
          filename: input.filename,
          parent_folder_id: input.parentFolderId ?? null,
          content_type: input.contentType,
          size_bytes: input.sizeBytes,
          checksum_sha256: null,
          part_size_bytes: input.partSizeBytes ?? null,
        },
      })
      .then((response) => response.body)
  }
)

declare global {
  namespace Cypress {
    interface Chainable {
      api<T = unknown>(
        method: string,
        path: string,
        options?: ApiOptions
      ): Chainable<Response<T>>
      signupByApi(email: string, password?: string): Chainable<AuthResponse>
      loginByApi(email: string, password?: string): Chainable<AuthResponse>
      authenticatedVisit(path: string, token: string): Chainable<AUTWindow>
      seedCompletedFile(
        token: string,
        input: SeedCompletedFileInput
      ): Chainable<{ fileId: string; objectKey: string }>
      createResumableSession(
        token: string,
        input: CreateResumableSessionInput
      ): Chainable<CreateResumableUploadResponse>
    }
  }
}

export {}
