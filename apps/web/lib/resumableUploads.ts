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

// Progress reported by the server for a resumable session: the source of truth
// for existence and expiration, plus how many parts already landed.
export type ServerPendingUpload = {
  file_id: string
  filename: string
  parent_folder_id: string | null
  size_bytes: number
  part_size_bytes: number
  checksum_sha256: string | null
  parts_received: number
  expires_at: string
}

export type PendingResumableUpload = {
  key: string
  upload: StoredResumableUpload
  // Present when the server still knows this session (merged pending list).
  server?: ServerPendingUpload
}

export type ResumedProgress = {
  partsDone: number
  partsTotal: number
  bytesDone: number
  percent: number
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

// Merges the server's authoritative pending list with the local key map. The
// server decides which sessions exist and when they expire; localStorage only
// contributes the resumableUploadKey that ties a local file to a session.
// Sessions the server no longer reports are dropped from localStorage.
export function mergePendingUploads(
  serverUploads: ServerPendingUpload[],
  storage: StorageLike | null = browserStorage()
): PendingResumableUpload[] {
  const stored = readStoredUploads(storage)
  const localByFileId = new Map<string, { key: string; upload: StoredResumableUpload }>()
  for (const [key, upload] of Object.entries(stored)) {
    localByFileId.set(upload.file_id, { key, upload })
  }

  const serverFileIds = new Set(serverUploads.map((upload) => upload.file_id))
  let changed = false
  for (const [key, upload] of Object.entries(stored)) {
    if (!serverFileIds.has(upload.file_id)) {
      delete stored[key]
      changed = true
    }
  }
  if (changed) {
    writeStoredUploads(stored, storage)
  }

  return serverUploads
    .map((server) => {
      const local = localByFileId.get(server.file_id)
      const key = local?.key ?? server.file_id
      const upload: StoredResumableUpload = {
        file_id: server.file_id,
        filename: local?.upload.filename ?? server.filename,
        size_bytes: local?.upload.size_bytes ?? server.size_bytes,
        last_modified: local?.upload.last_modified ?? 0,
        content_type:
          local?.upload.content_type ?? "application/octet-stream",
        parent_folder_id:
          local?.upload.parent_folder_id ?? server.parent_folder_id,
        // Server wins on expiration.
        expires_at: server.expires_at,
      }
      return { key, upload, server }
    })
    .sort((left, right) =>
      left.upload.filename.localeCompare(right.upload.filename)
    )
}

// Removes local entries that are expired locally, or that the server no longer
// reports as active (i.e. expired/purged server-side). Pass null for the active
// set when the server list is unavailable to prune by local expiry only.
export function clearExpiredUploads(
  activeServerFileIds: Set<string> | null = null,
  nowMs = Date.now(),
  storage: StorageLike | null = browserStorage()
): number {
  const stored = readStoredUploads(storage)
  let removed = 0
  for (const [key, upload] of Object.entries(stored)) {
    const expiredLocally = new Date(upload.expires_at).getTime() <= nowMs
    const goneServerSide =
      activeServerFileIds !== null && !activeServerFileIds.has(upload.file_id)
    if (expiredLocally || goneServerSide) {
      delete stored[key]
      removed += 1
    }
  }
  if (removed > 0) {
    writeStoredUploads(stored, storage)
  }
  return removed
}

// Initial progress when resuming: the parts already confirmed server-side count
// as bytes done, so the bar picks up where the last session left off.
export function resumedProgress(
  confirmedParts: { part_number: number; size_bytes: number }[],
  sizeBytes: number,
  partSizeBytes: number
): ResumedProgress {
  const partsTotal =
    partSizeBytes > 0 ? Math.ceil(sizeBytes / partSizeBytes) : 0
  const bytesDone = confirmedParts.reduce((sum, part) => sum + part.size_bytes, 0)
  const percent =
    sizeBytes > 0 ? Math.round((bytesDone / sizeBytes) * 100) : 0
  return {
    partsDone: confirmedParts.length,
    partsTotal,
    bytesDone,
    percent,
  }
}
