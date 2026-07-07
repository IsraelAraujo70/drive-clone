import type { FileRecord, FolderRecord } from "@/lib/api"

export type DriveView = "my-drive" | "shared-with-me" | "trash"

export type DriveItem =
  { kind: "file"; item: FileRecord } | { kind: "folder"; item: FolderRecord }
