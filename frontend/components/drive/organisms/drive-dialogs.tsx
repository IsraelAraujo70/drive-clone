import { CreateFolderDialog } from "@/components/drive/organisms/dialogs/create-folder-dialog"
import { MoveDialog } from "@/components/drive/organisms/dialogs/move-dialog"
import { RenameDialog } from "@/components/drive/organisms/dialogs/rename-dialog"
import { ShareDialog } from "@/components/drive/organisms/dialogs/share-dialog"
import type { DriveItem } from "@/components/drive/types"
import type { FileRecord, FolderRecord } from "@/lib/api"

export function DriveDialogs({
  allFolders,
  createFolderOpen,
  dialogError,
  dialogSubmitting,
  moveTarget,
  renameTarget,
  shareFile,
  token,
  onCreateFolder,
  onCreateFolderOpenChange,
  onDialogErrorChange,
  onMove,
  onMoveTargetChange,
  onRename,
  onRenameTargetChange,
  onShareFileChange,
}: {
  allFolders: FolderRecord[]
  createFolderOpen: boolean
  dialogError: string | null
  dialogSubmitting: boolean
  moveTarget: DriveItem | null
  renameTarget: DriveItem | null
  shareFile: FileRecord | null
  token: string | null
  onCreateFolder: (name: string) => void
  onCreateFolderOpenChange: (open: boolean) => void
  onDialogErrorChange: (error: string | null) => void
  onMove: (parentFolderId: string | null) => void
  onMoveTargetChange: (target: DriveItem | null) => void
  onRename: (name: string) => void
  onRenameTargetChange: (target: DriveItem | null) => void
  onShareFileChange: (file: FileRecord | null) => void
}) {
  return (
    <>
      <ShareDialog
        file={shareFile}
        token={token}
        open={Boolean(shareFile)}
        onOpenChange={(open) => {
          if (!open) {
            onShareFileChange(null)
          }
        }}
      />
      <CreateFolderDialog
        open={createFolderOpen}
        submitting={dialogSubmitting}
        error={dialogError}
        onOpenChange={(open) => {
          onCreateFolderOpenChange(open)
          onDialogErrorChange(null)
        }}
        onSubmit={onCreateFolder}
      />
      <RenameDialog
        target={renameTarget}
        submitting={dialogSubmitting}
        error={dialogError}
        onOpenChange={(open) => {
          if (!open) {
            onRenameTargetChange(null)
            onDialogErrorChange(null)
          }
        }}
        onSubmit={onRename}
      />
      <MoveDialog
        target={moveTarget}
        folders={allFolders}
        submitting={dialogSubmitting}
        error={dialogError}
        onOpenChange={(open) => {
          if (!open) {
            onMoveTargetChange(null)
            onDialogErrorChange(null)
          }
        }}
        onSubmit={onMove}
      />
    </>
  )
}
