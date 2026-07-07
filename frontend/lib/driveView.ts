import type { DriveView } from "@/components/drive/types"
import type { FileRecord, FolderRecord, SharedFileRecord } from "@/lib/api"

export function getVisibleDriveItems(input: {
  activeView: DriveView
  files: FileRecord[]
  folders: FolderRecord[]
  trashFiles: FileRecord[]
  trashFolders: FolderRecord[]
  sharedFiles: SharedFileRecord[]
}): {
  visibleFiles: Array<FileRecord | SharedFileRecord>
  visibleFolders: FolderRecord[]
} {
  const completedFiles = input.files.filter((file) => file.state === "complete")
  const completedTrashFiles = input.trashFiles.filter(
    (file) => file.state === "complete"
  )
  const completedSharedFiles = input.sharedFiles.filter(
    (file) => file.state === "complete"
  )

  if (input.activeView === "trash") {
    return {
      visibleFiles: completedTrashFiles,
      visibleFolders: input.trashFolders,
    }
  }
  if (input.activeView === "shared-with-me") {
    return {
      visibleFiles: completedSharedFiles,
      visibleFolders: [],
    }
  }
  return {
    visibleFiles: completedFiles,
    visibleFolders: input.folders,
  }
}
