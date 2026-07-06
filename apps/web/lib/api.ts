export const API_BASE_URL =
  process.env.NEXT_PUBLIC_API_BASE_URL ??
  (process.env.NODE_ENV === "development"
    ? "http://localhost:8080"
    : "https://api-production-bcad4.up.railway.app")

export type User = {
  id: string
  email: string
  display_name: string
  storage_quota_bytes: number
  storage_used_bytes: number
  created_at: string
}

export type AuthResponse = {
  user: User
  token: string
}

export type FileRecord = {
  id: string
  filename: string
  parent_folder_id: string | null
  content_type: string
  size_bytes: number
  checksum_sha256: string | null
  object_key: string
  state: "pending" | "complete"
  created_at: string
  updated_at: string
  completed_at: string | null
  deleted_at: string | null
}

export type FolderRecord = {
  id: string
  name: string
  parent_folder_id: string | null
  created_at: string
  updated_at: string
  deleted_at: string | null
}

export type FolderPathEntry = {
  id: string
  name: string
}

export type ShareUser = {
  id: string
  email: string
  display_name: string
}

export type Share = {
  file_id: string
  grantee: ShareUser
  created_at: string
}

export type SharedFileRecord = FileRecord & {
  owner: ShareUser
}

export type SearchAccess = "owned" | "shared"

export type SearchFileResult = {
  access: SearchAccess
  file: FileRecord
  owner: ShareUser | null
}

export type CreateUploadInput = {
  filename: string
  parent_folder_id?: string | null
  content_type: string
  size_bytes: number
  checksum_sha256?: string | null
}

export type CreateUploadResponse = {
  file_id: string
  upload_url: string
  object_key: string
  expires_at: string
}

export type CreateResumableUploadInput = CreateUploadInput & {
  part_size_bytes?: number | null
}

export type CreateResumableUploadResponse = {
  file_id: string
  object_key: string
  part_size_bytes: number
  expires_at: string
}

export type UploadPartRecord = {
  part_number: number
  size_bytes: number
  etag: string
}

export type UploadStatusResponse = {
  file_id: string
  filename: string
  parent_folder_id: string | null
  content_type: string
  size_bytes: number
  checksum_sha256: string | null
  object_key: string
  state: "pending" | "complete" | "expired"
  part_size_bytes: number
  expires_at: string
  created_at: string
  updated_at: string
  completed_at: string | null
  parts: UploadPartRecord[]
}

export type PresignUploadPartResponse = {
  file_id: string
  part_number: number
  upload_url: string
  expires_at: string
  expected_size_bytes: number
}

export type PendingUploadRecord = {
  file_id: string
  filename: string
  parent_folder_id: string | null
  size_bytes: number
  part_size_bytes: number
  checksum_sha256: string | null
  parts_received: number
  expires_at: string
}

export type ListPendingUploadsResponse = {
  uploads: PendingUploadRecord[]
}

export type ListFilesResponse = {
  files: FileRecord[]
}

export type DriveBrowseResponse = {
  parent_folder_id: string | null
  breadcrumbs: FolderPathEntry[]
  folders: FolderRecord[]
  files: FileRecord[]
}

export type ListFoldersResponse = {
  folders: FolderRecord[]
}

export type ListSharedFilesResponse = {
  files: SharedFileRecord[]
}

export type SearchFilesResponse = {
  query: string
  files: SearchFileResult[]
}

export type ListSharesResponse = {
  shares: Share[]
}

export type DownloadResponse = {
  download_url: string
  expires_at: string
}

export type DirectUploadProgress = {
  loaded: number
  total: number
  percent: number
}

export class ApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string
  ) {
    super(message)
    this.name = "ApiError"
  }
}

// Thrown while PUTting a single multipart part to object storage, so the UI can
// distinguish a transient network/storage failure from an API-level error.
export class UploadPartError extends Error {
  constructor(
    public partNumber: number,
    public reason: "network" | "http",
    public status?: number
  ) {
    super(
      reason === "http"
        ? `Upload part ${partNumber} failed with status ${status ?? "unknown"}`
        : `Upload part ${partNumber} failed`
    )
    this.name = "UploadPartError"
  }
}

type RequestOptions = {
  method?: string
  token?: string | null
  body?: unknown
}

async function request<T>(
  path: string,
  options: RequestOptions = {}
): Promise<T> {
  const headers: Record<string, string> = {}
  if (options.body !== undefined) {
    headers["Content-Type"] = "application/json"
  }
  if (options.token) {
    headers.Authorization = `Bearer ${options.token}`
  }

  const response = await fetch(`${API_BASE_URL}${path}`, {
    method: options.method ?? "GET",
    headers,
    body: options.body !== undefined ? JSON.stringify(options.body) : undefined,
  })

  if (response.status === 204) {
    return undefined as T
  }

  const data = await response.json().catch(() => null)
  if (!response.ok) {
    throw new ApiError(
      response.status,
      data?.error ?? "unknown_error",
      data?.message ?? "Something went wrong. Try again."
    )
  }
  return data as T
}

export function uploadFileDirect(
  uploadUrl: string,
  file: File,
  onProgress?: (progress: DirectUploadProgress) => void
): Promise<void> {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest()

    xhr.upload.addEventListener("progress", (event) => {
      if (!event.lengthComputable) {
        return
      }

      onProgress?.({
        loaded: event.loaded,
        total: event.total,
        percent: Math.round((event.loaded / event.total) * 100),
      })
    })

    xhr.addEventListener("load", () => {
      if (xhr.status >= 200 && xhr.status < 400) {
        resolve()
        return
      }

      reject(new Error(`Direct upload failed with status ${xhr.status}`))
    })

    xhr.addEventListener("error", () => {
      reject(new Error("Direct upload failed"))
    })

    xhr.addEventListener("abort", () => {
      reject(new Error("Direct upload aborted"))
    })

    xhr.open("PUT", uploadUrl)
    xhr.setRequestHeader(
      "Content-Type",
      file.type || "application/octet-stream"
    )
    xhr.send(file)
  })
}

export function uploadFilePart(
  uploadUrl: string,
  blob: Blob,
  partNumber: number,
  onProgress?: (loaded: number, total: number) => void
): Promise<string> {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest()

    xhr.upload.addEventListener("progress", (event) => {
      if (event.lengthComputable) {
        onProgress?.(event.loaded, event.total)
      }
    })

    xhr.addEventListener("load", () => {
      if (xhr.status >= 200 && xhr.status < 400) {
        const etag = xhr.getResponseHeader("ETag")
        if (!etag) {
          reject(
            new Error("Upload part completed without an ETag response header")
          )
          return
        }
        resolve(etag)
        return
      }

      reject(new UploadPartError(partNumber, "http", xhr.status))
    })

    xhr.addEventListener("error", () => {
      reject(new UploadPartError(partNumber, "network"))
    })

    xhr.addEventListener("abort", () => {
      reject(new UploadPartError(partNumber, "network"))
    })

    xhr.open("PUT", uploadUrl)
    xhr.send(blob)
  })
}

export const api = {
  signup: (input: { email: string; password: string; display_name: string }) =>
    request<AuthResponse>("/auth/signup", { method: "POST", body: input }),
  login: (input: { email: string; password: string }) =>
    request<AuthResponse>("/auth/login", { method: "POST", body: input }),
  logout: (token: string) =>
    request<void>("/auth/logout", { method: "POST", token }),
  me: (token: string) => request<User>("/auth/me", { token }),
  createUpload: (token: string, input: CreateUploadInput) =>
    request<CreateUploadResponse>("/files/uploads", {
      method: "POST",
      token,
      body: input,
    }),
  createResumableUpload: (token: string, input: CreateResumableUploadInput) =>
    request<CreateResumableUploadResponse>("/files/uploads/resumable", {
      method: "POST",
      token,
      body: input,
    }),
  getUploadStatus: (token: string, fileId: string) =>
    request<UploadStatusResponse>(`/files/uploads/${fileId}/status`, { token }),
  listPendingUploads: (token: string) =>
    request<ListPendingUploadsResponse>("/files/uploads/pending", { token }),
  presignUploadPart: (token: string, fileId: string, partNumber: number) =>
    request<PresignUploadPartResponse>(`/files/uploads/${fileId}/parts`, {
      method: "POST",
      token,
      body: { part_number: partNumber },
    }),
  recordUploadPart: (
    token: string,
    fileId: string,
    partNumber: number,
    input: { size_bytes: number; etag: string }
  ) =>
    request<UploadPartRecord>(`/files/uploads/${fileId}/parts/${partNumber}`, {
      method: "POST",
      token,
      body: input,
    }),
  finalizeResumableUpload: (token: string, fileId: string) =>
    request<FileRecord>(`/files/uploads/${fileId}/finalize`, {
      method: "POST",
      token,
    }),
  createFolder: (
    token: string,
    input: { name: string; parent_folder_id?: string | null }
  ) =>
    request<FolderRecord>("/folders", {
      method: "POST",
      token,
      body: input,
    }),
  browseDrive: (token: string, parentFolderId?: string | null) => {
    const query = parentFolderId
      ? `?parent_folder_id=${encodeURIComponent(parentFolderId)}`
      : ""
    return request<DriveBrowseResponse>(`/drive${query}`, { token })
  },
  listFolders: (token: string) =>
    request<ListFoldersResponse>("/folders", { token }),
  completeUpload: (token: string, fileId: string) =>
    request<FileRecord>(`/files/${fileId}/complete`, { method: "POST", token }),
  listFiles: (token: string) => request<ListFilesResponse>("/files", { token }),
  updateFile: (
    token: string,
    fileId: string,
    input: { filename?: string; parent_folder_id?: string | null }
  ) =>
    request<FileRecord>(`/files/${fileId}`, {
      method: "PATCH",
      token,
      body: input,
    }),
  updateFolder: (
    token: string,
    folderId: string,
    input: { name?: string; parent_folder_id?: string | null }
  ) =>
    request<FolderRecord>(`/folders/${folderId}`, {
      method: "PATCH",
      token,
      body: input,
    }),
  deleteFile: (token: string, fileId: string) =>
    request<void>(`/files/${fileId}`, { method: "DELETE", token }),
  deleteFolder: (token: string, folderId: string) =>
    request<void>(`/folders/${folderId}`, { method: "DELETE", token }),
  restoreFile: (token: string, fileId: string) =>
    request<FileRecord>(`/files/${fileId}/restore`, { method: "POST", token }),
  restoreFolder: (token: string, folderId: string) =>
    request<FolderRecord>(`/folders/${folderId}/restore`, {
      method: "POST",
      token,
    }),
  listTrash: (token: string) =>
    request<ListFilesResponse>("/files/trash", { token }),
  listDriveTrash: (token: string) =>
    request<DriveBrowseResponse>("/drive/trash", { token }),
  shareFile: (token: string, fileId: string, email: string) =>
    request<Share>(`/files/${fileId}/shares`, {
      method: "POST",
      token,
      body: { email },
    }),
  listShares: (token: string, fileId: string) =>
    request<ListSharesResponse>(`/files/${fileId}/shares`, { token }),
  revokeShare: (token: string, fileId: string, granteeId: string) =>
    request<void>(`/files/${fileId}/shares/${granteeId}`, {
      method: "DELETE",
      token,
    }),
  listSharedWithMe: (token: string) =>
    request<ListSharedFilesResponse>("/files/shared-with-me", { token }),
  searchFiles: (
    token: string,
    query: string,
    options: { include_deleted?: boolean; limit?: number } = {}
  ) => {
    const trimmed = query.trim()
    if (!trimmed) {
      return Promise.resolve({
        query: trimmed,
        files: [],
      } satisfies SearchFilesResponse)
    }

    const params = [`q=${encodeURIComponent(trimmed)}`]
    if (options.include_deleted !== undefined) {
      params.push(`include_deleted=${String(options.include_deleted)}`)
    }
    if (options.limit !== undefined) {
      params.push(`limit=${String(options.limit)}`)
    }

    return request<SearchFilesResponse>(`/search?${params.join("&")}`, {
      token,
    })
  },
  createDownload: (token: string, fileId: string) =>
    request<DownloadResponse>(`/files/${fileId}/download`, { token }),
  health: () => request<{ status: string; service: string }>("/health"),
}
