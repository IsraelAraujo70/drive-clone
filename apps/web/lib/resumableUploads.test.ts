import { describe, expect, it } from "vitest"

import {
  clearExpiredUploads,
  fileMatchesStoredUpload,
  forgetUpload,
  mergePendingUploads,
  pendingStoredUploads,
  readStoredUploads,
  rememberUpload,
  resumableUploadKey,
  resumedProgress,
  type ServerPendingUpload,
  type StoredResumableUpload,
} from "./resumableUploads"

class MemoryStorage {
  values = new Map<string, string>()

  getItem(key: string) {
    return this.values.get(key) ?? null
  }

  setItem(key: string, value: string) {
    this.values.set(key, value)
  }
}

function file(name = "BKEEPER.dmg") {
  return new File(["bytes"], name, {
    type: "application/x-apple-diskimage",
    lastModified: 1_720_000_000_000,
  })
}

function upload(overrides: Partial<StoredResumableUpload> = {}) {
  return {
    file_id: "file-1",
    filename: "BKEEPER.dmg",
    size_bytes: 5,
    last_modified: 1_720_000_000_000,
    content_type: "application/x-apple-diskimage",
    parent_folder_id: null,
    expires_at: "2026-07-07T00:00:00.000Z",
    ...overrides,
  } satisfies StoredResumableUpload
}

function serverUpload(
  overrides: Partial<ServerPendingUpload> = {}
): ServerPendingUpload {
  return {
    file_id: "file-1",
    filename: "BKEEPER.dmg",
    parent_folder_id: null,
    size_bytes: 5,
    part_size_bytes: 5,
    checksum_sha256: null,
    parts_received: 0,
    expires_at: "2026-07-08T00:00:00.000Z",
    ...overrides,
  }
}

describe("resumable upload storage", () => {
  it("stores, lists, and removes pending uploads", () => {
    const storage = new MemoryStorage()
    const key = resumableUploadKey(file(), null)

    rememberUpload(key, upload(), storage)

    expect(readStoredUploads(storage)[key]).toEqual(upload())
    expect(
      pendingStoredUploads(Date.parse("2026-07-06T00:00:00.000Z"), storage)
    ).toEqual([{ key, upload: upload() }])

    forgetUpload(key, storage)

    expect(
      pendingStoredUploads(Date.parse("2026-07-06T00:00:00.000Z"), storage)
    ).toEqual([])
  })

  it("prunes expired uploads when listing pending uploads", () => {
    const storage = new MemoryStorage()
    const expiredKey = resumableUploadKey(file("expired.dmg"), null)
    const activeKey = resumableUploadKey(file(), null)

    rememberUpload(
      expiredKey,
      upload({
        filename: "expired.dmg",
        expires_at: "2026-07-05T00:00:00.000Z",
      }),
      storage
    )
    rememberUpload(activeKey, upload(), storage)

    expect(
      pendingStoredUploads(Date.parse("2026-07-06T00:00:00.000Z"), storage)
    ).toEqual([{ key: activeKey, upload: upload() }])
    expect(readStoredUploads(storage)).not.toHaveProperty(expiredKey)
  })

  it("matches only the same local file for a saved upload", () => {
    const key = resumableUploadKey(file(), "folder-1")
    const pending = {
      key,
      upload: upload({ parent_folder_id: "folder-1" }),
    }

    expect(fileMatchesStoredUpload(file(), pending)).toBe(true)
    expect(
      fileMatchesStoredUpload(
        new File(["bytes"], "BKEEPER.dmg", {
          type: "",
          lastModified: 1_730_000_000_000,
        }),
        pending
      )
    ).toBe(true)
    expect(fileMatchesStoredUpload(file("other.dmg"), pending)).toBe(false)
  })
})

describe("mergePendingUploads", () => {
  it("keeps the local key but lets the server win on existence and expiration", () => {
    const storage = new MemoryStorage()
    const key = resumableUploadKey(file(), null)
    rememberUpload(key, upload(), storage)

    const merged = mergePendingUploads(
      [serverUpload({ parts_received: 2, expires_at: "2026-07-09T00:00:00.000Z" })],
      storage
    )

    expect(merged).toHaveLength(1)
    expect(merged[0].key).toBe(key)
    // Local metadata (last_modified, content_type) survives the merge...
    expect(merged[0].upload.last_modified).toBe(1_720_000_000_000)
    expect(merged[0].upload.content_type).toBe("application/x-apple-diskimage")
    // ...but the server's expiration wins.
    expect(merged[0].upload.expires_at).toBe("2026-07-09T00:00:00.000Z")
    expect(merged[0].server?.parts_received).toBe(2)
  })

  it("drops local sessions the server no longer reports", () => {
    const storage = new MemoryStorage()
    const goneKey = resumableUploadKey(file("gone.dmg"), null)
    rememberUpload(
      goneKey,
      upload({ file_id: "file-gone", filename: "gone.dmg" }),
      storage
    )

    const merged = mergePendingUploads([serverUpload()], storage)

    expect(merged.map((entry) => entry.upload.file_id)).toEqual(["file-1"])
    expect(readStoredUploads(storage)).not.toHaveProperty(goneKey)
  })

  it("still surfaces a server session without a local key", () => {
    const storage = new MemoryStorage()
    const merged = mergePendingUploads(
      [serverUpload({ file_id: "server-only", filename: "remote.iso" })],
      storage
    )

    expect(merged).toHaveLength(1)
    expect(merged[0].key).toBe("server-only")
    expect(merged[0].upload.filename).toBe("remote.iso")
  })
})

describe("clearExpiredUploads", () => {
  it("removes entries expired locally", () => {
    const storage = new MemoryStorage()
    const expiredKey = resumableUploadKey(file("old.dmg"), null)
    const activeKey = resumableUploadKey(file(), null)
    rememberUpload(
      expiredKey,
      upload({
        file_id: "file-old",
        filename: "old.dmg",
        expires_at: "2026-07-05T00:00:00.000Z",
      }),
      storage
    )
    rememberUpload(activeKey, upload(), storage)

    const removed = clearExpiredUploads(
      null,
      Date.parse("2026-07-06T00:00:00.000Z"),
      storage
    )

    expect(removed).toBe(1)
    expect(readStoredUploads(storage)).not.toHaveProperty(expiredKey)
    expect(readStoredUploads(storage)).toHaveProperty(activeKey)
  })

  it("removes entries the server reports as no longer active", () => {
    const storage = new MemoryStorage()
    const staleKey = resumableUploadKey(file("stale.dmg"), null)
    const activeKey = resumableUploadKey(file(), null)
    rememberUpload(
      staleKey,
      upload({ file_id: "file-stale", filename: "stale.dmg" }),
      storage
    )
    rememberUpload(activeKey, upload(), storage)

    const removed = clearExpiredUploads(
      new Set(["file-1"]),
      Date.parse("2026-07-06T00:00:00.000Z"),
      storage
    )

    expect(removed).toBe(1)
    expect(readStoredUploads(storage)).not.toHaveProperty(staleKey)
    expect(readStoredUploads(storage)).toHaveProperty(activeKey)
  })
})

describe("resumedProgress", () => {
  it("counts confirmed parts as bytes already done", () => {
    const progress = resumedProgress(
      [
        { part_number: 1, size_bytes: 5_000_000 },
        { part_number: 2, size_bytes: 5_000_000 },
      ],
      12_000_000,
      5_000_000
    )

    expect(progress.partsDone).toBe(2)
    expect(progress.partsTotal).toBe(3)
    expect(progress.bytesDone).toBe(10_000_000)
    expect(progress.percent).toBe(83)
  })

  it("is zeroed for a fresh upload", () => {
    const progress = resumedProgress([], 12_000_000, 5_000_000)
    expect(progress).toEqual({
      partsDone: 0,
      partsTotal: 3,
      bytesDone: 0,
      percent: 0,
    })
  })
})
