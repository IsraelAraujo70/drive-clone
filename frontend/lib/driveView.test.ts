import { describe, expect, it } from "vitest"

import type { FileRecord, FolderRecord, SharedFileRecord } from "./api"
import { getVisibleDriveItems } from "./driveView"

function file(id: string, state: FileRecord["state"] = "complete"): FileRecord {
  return {
    id,
    filename: `${id}.txt`,
    parent_folder_id: null,
    content_type: "text/plain",
    size_bytes: 10,
    checksum_sha256: null,
    object_key: id,
    state,
    created_at: "2026-01-01T00:00:00.000Z",
    updated_at: "2026-01-01T00:00:00.000Z",
    completed_at: state === "complete" ? "2026-01-01T00:00:00.000Z" : null,
    deleted_at: null,
  }
}

function folder(id: string): FolderRecord {
  return {
    id,
    name: id,
    parent_folder_id: null,
    created_at: "2026-01-01T00:00:00.000Z",
    updated_at: "2026-01-01T00:00:00.000Z",
    deleted_at: null,
  }
}

function shared(id: string, state: FileRecord["state"] = "complete") {
  return {
    ...file(id, state),
    owner: {
      id: "owner",
      email: "owner@example.com",
      display_name: "Owner",
    },
  } satisfies SharedFileRecord
}

describe("getVisibleDriveItems", () => {
  const input = {
    files: [file("owned-complete"), file("owned-pending", "pending")],
    folders: [folder("current-folder")],
    trashFiles: [file("trash-complete"), file("trash-pending", "pending")],
    trashFolders: [folder("trash-folder")],
    sharedFiles: [
      shared("shared-complete"),
      shared("shared-pending", "pending"),
    ],
  }

  it("returns complete owned files and current folders for my-drive", () => {
    const visible = getVisibleDriveItems({
      ...input,
      activeView: "my-drive",
    })

    expect(visible.visibleFiles.map((item) => item.id)).toEqual([
      "owned-complete",
    ])
    expect(visible.visibleFolders.map((item) => item.id)).toEqual([
      "current-folder",
    ])
  })

  it("returns complete trash files and trash folders for trash", () => {
    const visible = getVisibleDriveItems({
      ...input,
      activeView: "trash",
    })

    expect(visible.visibleFiles.map((item) => item.id)).toEqual([
      "trash-complete",
    ])
    expect(visible.visibleFolders.map((item) => item.id)).toEqual([
      "trash-folder",
    ])
  })

  it("returns complete shared files and no folders for shared-with-me", () => {
    const visible = getVisibleDriveItems({
      ...input,
      activeView: "shared-with-me",
    })

    expect(visible.visibleFiles.map((item) => item.id)).toEqual([
      "shared-complete",
    ])
    expect(visible.visibleFolders).toEqual([])
  })

  it("filters pending files out of every view", () => {
    for (const activeView of ["my-drive", "trash", "shared-with-me"] as const) {
      const visible = getVisibleDriveItems({ ...input, activeView })

      expect(
        visible.visibleFiles.every((item) => item.state === "complete")
      ).toBe(true)
    }
  })
})
