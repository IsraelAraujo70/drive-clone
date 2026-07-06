import { describe, expect, it } from "vitest"

import {
  fileMatchesStoredUpload,
  forgetUpload,
  pendingStoredUploads,
  readStoredUploads,
  rememberUpload,
  resumableUploadKey,
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
