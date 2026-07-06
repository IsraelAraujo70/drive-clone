const RESUMABLE_UPLOADS_KEY = "drive_clone_resumable_uploads_v1"

type StorageLike = Pick<Storage, "getItem" | "setItem">

export type StoredResumableUpload = {
  file_id: string
  filename: string
  size_bytes: number
  last_modified: number
  content_type: string
  parent_folder_id: string | null
  expires_at: string
}

export type PendingResumableUpload = {
  key: string
  upload: StoredResumableUpload
}

function browserStorage(): StorageLike | null {
  if (typeof window === "undefined") {
    return null
  }
  return window.localStorage
}

export function resumableUploadKey(file: File, parentFolderId: string | null) {
  return [
    file.name,
    file.size,
    file.lastModified,
    file.type || "application/octet-stream",
    parentFolderId ?? "root",
  ].join(":")
}

export function readStoredUploads(
  storage: StorageLike | null = browserStorage()
): Record<string, StoredResumableUpload> {
  if (!storage) {
    return {}
  }
  try {
    const parsed = JSON.parse(storage.getItem(RESUMABLE_UPLOADS_KEY) ?? "{}")
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      return {}
    }
    return parsed
  } catch {
    return {}
  }
}

export function writeStoredUploads(
  uploads: Record<string, StoredResumableUpload>,
  storage: StorageLike | null = browserStorage()
) {
  storage?.setItem(RESUMABLE_UPLOADS_KEY, JSON.stringify(uploads))
}

export function rememberUpload(
  key: string,
  upload: StoredResumableUpload,
  storage: StorageLike | null = browserStorage()
) {
  const uploads = readStoredUploads(storage)
  uploads[key] = upload
  writeStoredUploads(uploads, storage)
}

export function forgetUpload(
  key: string,
  storage: StorageLike | null = browserStorage()
) {
  const uploads = readStoredUploads(storage)
  delete uploads[key]
  writeStoredUploads(uploads, storage)
}

export function pendingStoredUploads(
  nowMs = Date.now(),
  storage: StorageLike | null = browserStorage()
): PendingResumableUpload[] {
  const uploads = readStoredUploads(storage)
  let changed = false
  const pending: PendingResumableUpload[] = []

  for (const [key, upload] of Object.entries(uploads)) {
    if (new Date(upload.expires_at).getTime() <= nowMs) {
      delete uploads[key]
      changed = true
      continue
    }
    pending.push({ key, upload })
  }

  if (changed) {
    writeStoredUploads(uploads, storage)
  }

  return pending.sort((left, right) =>
    left.upload.filename.localeCompare(right.upload.filename)
  )
}

export function fileMatchesStoredUpload(
  file: File,
  pending: PendingResumableUpload
) {
  return (
    file.name === pending.upload.filename &&
    file.size === pending.upload.size_bytes
  )
}
