import { describe, expect, it } from "vitest"

import type { FolderRecord } from "./api"
import { isDescendantFolder } from "./driveTree"

function folder(
  id: string,
  parent_folder_id: string | null = null
): FolderRecord {
  return {
    id,
    name: id,
    parent_folder_id,
    created_at: "2026-01-01T00:00:00.000Z",
    updated_at: "2026-01-01T00:00:00.000Z",
    deleted_at: null,
  }
}

describe("isDescendantFolder", () => {
  const folders = [
    folder("root-a"),
    folder("root-b"),
    folder("child-a", "root-a"),
    folder("grandchild-a", "child-a"),
    folder("child-b", "root-b"),
  ]

  it("returns true when the candidate parent is a direct child", () => {
    expect(isDescendantFolder(folders, "root-a", "child-a")).toBe(true)
  })

  it("returns true when the candidate parent is nested below the folder", () => {
    expect(isDescendantFolder(folders, "root-a", "grandchild-a")).toBe(true)
  })

  it("returns false for sibling folders", () => {
    expect(isDescendantFolder(folders, "child-a", "child-b")).toBe(false)
  })

  it("returns false for unrelated root folders", () => {
    expect(isDescendantFolder(folders, "root-a", "root-b")).toBe(false)
  })

  it("supports the caller guard that blocks moving a folder into itself", () => {
    const target = folder("root-a")
    const candidate = folder("root-a")
    const disabled =
      candidate.id === target.id ||
      isDescendantFolder(folders, target.id, candidate.id)

    expect(disabled).toBe(true)
  })
})
