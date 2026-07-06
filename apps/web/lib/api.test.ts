import { afterEach, describe, expect, it, vi } from "vitest"
import { API_BASE_URL, ApiError, api, uploadFileDirect } from "./api"

function jsonResponse(status: number, body: unknown) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  })
}

afterEach(() => {
  vi.unstubAllGlobals()
})

describe("api client", () => {
  it("posts signup input as JSON", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValue(jsonResponse(201, { user: { email: "a@b.co" }, token: "tok" }))
    vi.stubGlobal("fetch", fetchMock)

    const result = await api.signup({
      email: "a@b.co",
      password: "password123",
      display_name: "A",
    })

    expect(result.token).toBe("tok")
    const [url, init] = fetchMock.mock.calls[0]
    expect(url).toBe(`${API_BASE_URL}/auth/signup`)
    expect(init.method).toBe("POST")
    expect(init.headers["Content-Type"]).toBe("application/json")
    expect(JSON.parse(init.body)).toEqual({
      email: "a@b.co",
      password: "password123",
      display_name: "A",
    })
  })

  it("sends the bearer token on authenticated calls", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(200, { email: "a@b.co" }))
    vi.stubGlobal("fetch", fetchMock)

    await api.me("secret-token")

    const [url, init] = fetchMock.mock.calls[0]
    expect(url).toBe(`${API_BASE_URL}/auth/me`)
    expect(init.headers.Authorization).toBe("Bearer secret-token")
  })

  it("throws ApiError with the server error code and message", async () => {
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValue(
          jsonResponse(401, {
            error: "invalid_credentials",
            message: "Invalid email or password",
          }),
        ),
    )

    const error = await api
      .login({ email: "a@b.co", password: "wrong" })
      .catch((caught: unknown) => caught)

    expect(error).toBeInstanceOf(ApiError)
    expect((error as ApiError).status).toBe(401)
    expect((error as ApiError).code).toBe("invalid_credentials")
    expect((error as ApiError).message).toBe("Invalid email or password")
  })

  it("handles empty 204 responses", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(null, { status: 204 })))
    await expect(api.logout("secret-token")).resolves.toBeUndefined()
  })

  it("creates uploads with authenticated JSON metadata", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      jsonResponse(201, {
        file_id: "file-1",
        upload_url: "https://storage.example/upload",
        object_key: "objects/file-1",
        expires_at: "2026-07-02T12:00:00Z",
      }),
    )
    vi.stubGlobal("fetch", fetchMock)

    const result = await api.createUpload("secret-token", {
      filename: "report.pdf",
      parent_folder_id: "folder-1",
      content_type: "application/pdf",
      size_bytes: 42,
      checksum_sha256: null,
    })

    expect(result.file_id).toBe("file-1")
    const [url, init] = fetchMock.mock.calls[0]
    expect(url).toBe(`${API_BASE_URL}/files/uploads`)
    expect(init.method).toBe("POST")
    expect(init.headers.Authorization).toBe("Bearer secret-token")
    expect(init.headers["Content-Type"]).toBe("application/json")
    expect(JSON.parse(init.body)).toEqual({
      filename: "report.pdf",
      parent_folder_id: "folder-1",
      content_type: "application/pdf",
      size_bytes: 42,
      checksum_sha256: null,
    })
  })

  it("creates folders and browses root or folder locations", async () => {
    const folder = {
      id: "folder-1",
      name: "Projects",
      parent_folder_id: null,
      created_at: "2026-07-06T12:00:00Z",
      updated_at: "2026-07-06T12:00:00Z",
      deleted_at: null,
    }
    const browse = {
      parent_folder_id: "folder-1",
      breadcrumbs: [{ id: "folder-1", name: "Projects" }],
      folders: [],
      files: [],
    }
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse(201, folder))
      .mockResolvedValueOnce(jsonResponse(200, browse))
      .mockResolvedValueOnce(jsonResponse(200, { ...browse, parent_folder_id: null }))
      .mockResolvedValueOnce(jsonResponse(200, { folders: [folder] }))
    vi.stubGlobal("fetch", fetchMock)

    await expect(
      api.createFolder("secret-token", {
        name: "Projects",
        parent_folder_id: null,
      }),
    ).resolves.toEqual(folder)
    await expect(api.browseDrive("secret-token", "folder-1")).resolves.toEqual(
      browse,
    )
    await expect(api.browseDrive("secret-token", null)).resolves.toEqual({
      ...browse,
      parent_folder_id: null,
    })
    await expect(api.listFolders("secret-token")).resolves.toEqual({
      folders: [folder],
    })

    expect(fetchMock.mock.calls.map(([url]) => url)).toEqual([
      `${API_BASE_URL}/folders`,
      `${API_BASE_URL}/drive?parent_folder_id=folder-1`,
      `${API_BASE_URL}/drive`,
      `${API_BASE_URL}/folders`,
    ])
    expect(JSON.parse(fetchMock.mock.calls[0][1].body)).toEqual({
      name: "Projects",
      parent_folder_id: null,
    })
  })

  it("renames, moves, deletes, restores folders and lists drive trash", async () => {
    const folder = {
      id: "folder-1",
      name: "Projects",
      parent_folder_id: null,
      created_at: "2026-07-06T12:00:00Z",
      updated_at: "2026-07-06T12:00:00Z",
      deleted_at: null,
    }
    const file = {
      id: "file-1",
      filename: "report.pdf",
      parent_folder_id: "folder-1",
      content_type: "application/pdf",
      size_bytes: 42,
      checksum_sha256: null,
      object_key: "objects/file-1",
      state: "complete",
      created_at: "2026-07-02T12:00:00Z",
      updated_at: "2026-07-02T12:01:00Z",
      completed_at: "2026-07-02T12:01:00Z",
      deleted_at: null,
    }
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse(200, { ...file, filename: "renamed.pdf" }))
      .mockResolvedValueOnce(jsonResponse(200, { ...folder, name: "Work" }))
      .mockResolvedValueOnce(new Response(null, { status: 204 }))
      .mockResolvedValueOnce(jsonResponse(200, folder))
      .mockResolvedValueOnce(
        jsonResponse(200, {
          parent_folder_id: null,
          breadcrumbs: [],
          folders: [folder],
          files: [],
        }),
      )
    vi.stubGlobal("fetch", fetchMock)

    await expect(
      api.updateFile("secret-token", "file-1", {
        filename: "renamed.pdf",
        parent_folder_id: null,
      }),
    ).resolves.toMatchObject({ filename: "renamed.pdf" })
    await expect(
      api.updateFolder("secret-token", "folder-1", {
        name: "Work",
        parent_folder_id: null,
      }),
    ).resolves.toMatchObject({ name: "Work" })
    await expect(api.deleteFolder("secret-token", "folder-1")).resolves.toBeUndefined()
    await expect(api.restoreFolder("secret-token", "folder-1")).resolves.toEqual(
      folder,
    )
    await expect(api.listDriveTrash("secret-token")).resolves.toEqual({
      parent_folder_id: null,
      breadcrumbs: [],
      folders: [folder],
      files: [],
    })

    expect(fetchMock.mock.calls.map(([url]) => url)).toEqual([
      `${API_BASE_URL}/files/file-1`,
      `${API_BASE_URL}/folders/folder-1`,
      `${API_BASE_URL}/folders/folder-1`,
      `${API_BASE_URL}/folders/folder-1/restore`,
      `${API_BASE_URL}/drive/trash`,
    ])
    expect(fetchMock.mock.calls[0][1].method).toBe("PATCH")
    expect(JSON.parse(fetchMock.mock.calls[0][1].body)).toEqual({
      filename: "renamed.pdf",
      parent_folder_id: null,
    })
    expect(fetchMock.mock.calls[2][1].method).toBe("DELETE")
  })

  it("completes uploads, lists files, and requests download URLs", async () => {
    const completedFile = {
      id: "file-1",
      filename: "report.pdf",
      parent_folder_id: null,
      content_type: "application/pdf",
      size_bytes: 42,
      checksum_sha256: null,
      object_key: "objects/file-1",
      state: "complete",
      created_at: "2026-07-02T12:00:00Z",
      updated_at: "2026-07-02T12:01:00Z",
      completed_at: "2026-07-02T12:01:00Z",
      deleted_at: null,
    }
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse(200, completedFile))
      .mockResolvedValueOnce(jsonResponse(200, { files: [completedFile] }))
      .mockResolvedValueOnce(
        jsonResponse(200, {
          download_url: "https://storage.example/download",
          expires_at: "2026-07-02T13:00:00Z",
        }),
      )
    vi.stubGlobal("fetch", fetchMock)

    await expect(api.completeUpload("secret-token", "file-1")).resolves.toEqual(
      completedFile,
    )
    await expect(api.listFiles("secret-token")).resolves.toEqual({
      files: [completedFile],
    })
    await expect(api.createDownload("secret-token", "file-1")).resolves.toEqual({
      download_url: "https://storage.example/download",
      expires_at: "2026-07-02T13:00:00Z",
    })

    expect(fetchMock.mock.calls.map(([url]) => url)).toEqual([
      `${API_BASE_URL}/files/file-1/complete`,
      `${API_BASE_URL}/files`,
      `${API_BASE_URL}/files/file-1/download`,
    ])
    expect(fetchMock.mock.calls[0][1].method).toBe("POST")
    expect(fetchMock.mock.calls[1][1].headers.Authorization).toBe(
      "Bearer secret-token",
    )
  })

  it("soft deletes, restores, and lists trash files", async () => {
    const trashFile = {
      id: "file-1",
      filename: "report.pdf",
      parent_folder_id: null,
      content_type: "application/pdf",
      size_bytes: 42,
      checksum_sha256: null,
      object_key: "objects/file-1",
      state: "complete",
      created_at: "2026-07-02T12:00:00Z",
      updated_at: "2026-07-03T12:00:00Z",
      completed_at: "2026-07-02T12:01:00Z",
      deleted_at: "2026-07-03T12:00:00Z",
    }
    const restoredFile = { ...trashFile, deleted_at: null }
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(new Response(null, { status: 204 }))
      .mockResolvedValueOnce(jsonResponse(200, restoredFile))
      .mockResolvedValueOnce(jsonResponse(200, { files: [trashFile] }))
    vi.stubGlobal("fetch", fetchMock)

    await expect(api.deleteFile("secret-token", "file-1")).resolves.toBeUndefined()
    await expect(api.restoreFile("secret-token", "file-1")).resolves.toEqual(
      restoredFile,
    )
    await expect(api.listTrash("secret-token")).resolves.toEqual({
      files: [trashFile],
    })

    expect(fetchMock.mock.calls.map(([url]) => url)).toEqual([
      `${API_BASE_URL}/files/file-1`,
      `${API_BASE_URL}/files/file-1/restore`,
      `${API_BASE_URL}/files/trash`,
    ])
    expect(fetchMock.mock.calls[0][1].method).toBe("DELETE")
    expect(fetchMock.mock.calls[1][1].method).toBe("POST")
    expect(fetchMock.mock.calls[2][1].headers.Authorization).toBe(
      "Bearer secret-token",
    )
  })

  it("creates, lists, and revokes file shares", async () => {
    const share = {
      file_id: "file-1",
      grantee: {
        id: "user-2",
        email: "friend@example.com",
        display_name: "Friend",
      },
      created_at: "2026-07-03T12:00:00Z",
    }
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse(201, share))
      .mockResolvedValueOnce(jsonResponse(200, { shares: [share] }))
      .mockResolvedValueOnce(new Response(null, { status: 204 }))
    vi.stubGlobal("fetch", fetchMock)

    await expect(
      api.shareFile("secret-token", "file-1", "friend@example.com"),
    ).resolves.toEqual(share)
    await expect(api.listShares("secret-token", "file-1")).resolves.toEqual({
      shares: [share],
    })
    await expect(
      api.revokeShare("secret-token", "file-1", "user-2"),
    ).resolves.toBeUndefined()

    expect(fetchMock.mock.calls.map(([url]) => url)).toEqual([
      `${API_BASE_URL}/files/file-1/shares`,
      `${API_BASE_URL}/files/file-1/shares`,
      `${API_BASE_URL}/files/file-1/shares/user-2`,
    ])
    expect(fetchMock.mock.calls[0][1].method).toBe("POST")
    expect(fetchMock.mock.calls[0][1].headers["Content-Type"]).toBe(
      "application/json",
    )
    expect(JSON.parse(fetchMock.mock.calls[0][1].body)).toEqual({
      email: "friend@example.com",
    })
    expect(fetchMock.mock.calls[1][1].method).toBe("GET")
    expect(fetchMock.mock.calls[2][1].method).toBe("DELETE")
  })

  it("lists files shared with the current user", async () => {
    const sharedFile = {
      id: "file-1",
      filename: "report.pdf",
      parent_folder_id: null,
      content_type: "application/pdf",
      size_bytes: 42,
      checksum_sha256: null,
      object_key: "objects/file-1",
      state: "complete",
      created_at: "2026-07-02T12:00:00Z",
      updated_at: "2026-07-02T12:01:00Z",
      completed_at: "2026-07-02T12:01:00Z",
      deleted_at: null,
      owner: {
        id: "owner-1",
        email: "owner@example.com",
        display_name: "Owner",
      },
    }
    const fetchMock = vi
      .fn()
      .mockResolvedValue(jsonResponse(200, { files: [sharedFile] }))
    vi.stubGlobal("fetch", fetchMock)

    await expect(api.listSharedWithMe("secret-token")).resolves.toEqual({
      files: [sharedFile],
    })

    const [url, init] = fetchMock.mock.calls[0]
    expect(url).toBe(`${API_BASE_URL}/files/shared-with-me`)
    expect(init.headers.Authorization).toBe("Bearer secret-token")
  })

  it("surfaces user_not_found ApiError when sharing with an unknown email", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        jsonResponse(404, {
          error: "user_not_found",
          message: "No user exists for that email.",
        }),
      ),
    )

    const error = await api
      .shareFile("secret-token", "file-1", "missing@example.com")
      .catch((caught: unknown) => caught)

    expect(error).toBeInstanceOf(ApiError)
    expect((error as ApiError).status).toBe(404)
    expect((error as ApiError).code).toBe("user_not_found")
    expect((error as ApiError).message).toBe("No user exists for that email.")
  })

  it("falls back to a generic error on non-JSON failures", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response("boom", { status: 500 })))

    const error = await api.health().catch((caught: unknown) => caught)
    expect(error).toBeInstanceOf(ApiError)
    expect((error as ApiError).code).toBe("unknown_error")
  })
})

type MockXhrEvent = {
  lengthComputable?: boolean
  loaded?: number
  total?: number
}

class MockXMLHttpRequest {
  static instances: MockXMLHttpRequest[] = []

  status = 0
  method = ""
  url = ""
  body: BodyInit | null = null
  headers: Record<string, string> = {}
  uploadListeners: Record<string, ((event: MockXhrEvent) => void)[]> = {}
  listeners: Record<string, (() => void)[]> = {}

  upload = {
    addEventListener: (type: string, listener: (event: MockXhrEvent) => void) => {
      this.uploadListeners[type] = [...(this.uploadListeners[type] ?? []), listener]
    },
  }

  constructor() {
    MockXMLHttpRequest.instances.push(this)
  }

  open(method: string, url: string) {
    this.method = method
    this.url = url
  }

  setRequestHeader(name: string, value: string) {
    this.headers[name] = value
  }

  send(body: BodyInit) {
    this.body = body
  }

  addEventListener(type: string, listener: () => void) {
    this.listeners[type] = [...(this.listeners[type] ?? []), listener]
  }

  emitUploadProgress(event: Required<MockXhrEvent>) {
    for (const listener of this.uploadListeners.progress ?? []) {
      listener(event)
    }
  }

  emit(type: string) {
    for (const listener of this.listeners[type] ?? []) {
      listener()
    }
  }
}

describe("direct upload helper", () => {
  afterEach(() => {
    MockXMLHttpRequest.instances = []
  })

  it("puts raw file bytes and reports computable progress", async () => {
    vi.stubGlobal("XMLHttpRequest", MockXMLHttpRequest)
    const file = new File(["hello"], "hello.txt", { type: "text/plain" })
    const onProgress = vi.fn()

    const upload = uploadFileDirect(
      "https://storage.example/upload",
      file,
      onProgress,
    )

    const xhr = MockXMLHttpRequest.instances[0]
    expect(xhr.method).toBe("PUT")
    expect(xhr.url).toBe("https://storage.example/upload")
    expect(xhr.headers["Content-Type"]).toBe("text/plain")
    expect(xhr.body).toBe(file)

    xhr.emitUploadProgress({ lengthComputable: true, loaded: 3, total: 5 })
    xhr.status = 204
    xhr.emit("load")

    await expect(upload).resolves.toBeUndefined()
    expect(onProgress).toHaveBeenCalledWith({ loaded: 3, total: 5, percent: 60 })
  })

  it("uses application/octet-stream when the file has no type", async () => {
    vi.stubGlobal("XMLHttpRequest", MockXMLHttpRequest)
    const file = new File(["hello"], "hello.bin")

    const upload = uploadFileDirect("https://storage.example/upload", file)
    const xhr = MockXMLHttpRequest.instances[0]

    expect(xhr.headers["Content-Type"]).toBe("application/octet-stream")
    xhr.status = 200
    xhr.emit("load")
    await expect(upload).resolves.toBeUndefined()
  })

  it("rejects non-2xx and non-3xx XHR statuses", async () => {
    vi.stubGlobal("XMLHttpRequest", MockXMLHttpRequest)
    const file = new File(["hello"], "hello.txt", { type: "text/plain" })

    const upload = uploadFileDirect("https://storage.example/upload", file)
    const xhr = MockXMLHttpRequest.instances[0]
    xhr.status = 403
    xhr.emit("load")

    await expect(upload).rejects.toThrow("Direct upload failed with status 403")
  })

  it("ignores non-computable progress events", async () => {
    vi.stubGlobal("XMLHttpRequest", MockXMLHttpRequest)
    const file = new File(["hello"], "hello.txt", { type: "text/plain" })
    const onProgress = vi.fn()

    const upload = uploadFileDirect(
      "https://storage.example/upload",
      file,
      onProgress,
    )
    const xhr = MockXMLHttpRequest.instances[0]
    xhr.emitUploadProgress({ lengthComputable: false, loaded: 3, total: 5 })
    xhr.status = 200
    xhr.emit("load")

    await expect(upload).resolves.toBeUndefined()
    expect(onProgress).not.toHaveBeenCalled()
  })
})
