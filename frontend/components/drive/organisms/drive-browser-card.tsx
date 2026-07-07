import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Spinner } from "@/components/ui/spinner"
import type { viewCopy } from "@/components/drive/constants"
import { DriveEmptyState } from "@/components/drive/molecules/drive-empty-state"
import { DriveFileRow } from "@/components/drive/molecules/drive-file-row"
import { DriveFolderRow } from "@/components/drive/molecules/drive-folder-row"
import { DriveToolbar } from "@/components/drive/molecules/drive-toolbar"
import type { DriveView } from "@/components/drive/types"
import type { FileRecord, FolderRecord, SharedFileRecord } from "@/lib/api"

export function DriveBrowserCard({
  activeView,
  copy,
  deleteId,
  downloadId,
  loadingFiles,
  purgeId,
  restoreId,
  uploadingName,
  visibleFiles,
  visibleFolders,
  onCreateFolder,
  onDeleteFile,
  onDeleteFolder,
  onDownload,
  onMoveFile,
  onMoveFolder,
  onOpenFolder,
  onPurgeFile,
  onPurgeFolder,
  onReload,
  onRenameFile,
  onRenameFolder,
  onRestoreFile,
  onRestoreFolder,
  onShareFile,
  onUpload,
}: {
  activeView: DriveView
  copy: (typeof viewCopy)[DriveView]
  deleteId: string | null
  downloadId: string | null
  loadingFiles: boolean
  purgeId: string | null
  restoreId: string | null
  uploadingName: string | null
  visibleFiles: Array<FileRecord | SharedFileRecord>
  visibleFolders: FolderRecord[]
  onCreateFolder: () => void
  onDeleteFile: (fileId: string) => void
  onDeleteFolder: (folderId: string) => void
  onDownload: (fileId: string) => void
  onMoveFile: (file: FileRecord) => void
  onMoveFolder: (folder: FolderRecord) => void
  onOpenFolder: (folderId: string) => void
  onPurgeFile: (file: FileRecord) => void
  onPurgeFolder: (folder: FolderRecord) => void
  onReload: () => void
  onRenameFile: (file: FileRecord) => void
  onRenameFolder: (folder: FolderRecord) => void
  onRestoreFile: (fileId: string) => void
  onRestoreFolder: (folderId: string) => void
  onShareFile: (file: FileRecord) => void
  onUpload: () => void
}) {
  return (
    <Card>
      <CardHeader>
        <CardDescription>{copy.cardDescription}</CardDescription>
        <CardTitle>{copy.cardTitle}</CardTitle>
        <CardAction>
          <DriveToolbar
            activeView={activeView}
            loadingFiles={loadingFiles}
            uploadingName={uploadingName}
            onCreateFolder={onCreateFolder}
            onReload={onReload}
            onUpload={onUpload}
          />
        </CardAction>
      </CardHeader>
      <CardContent>
        {loadingFiles ? (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Spinner />
            Loading files…
          </div>
        ) : visibleFiles.length === 0 && visibleFolders.length === 0 ? (
          <DriveEmptyState
            activeView={activeView}
            copy={copy}
            onUpload={onUpload}
          />
        ) : (
          <div className="flex flex-col gap-2">
            {visibleFolders.map((folder) => (
              <DriveFolderRow
                key={folder.id}
                activeView={activeView}
                deleteId={deleteId}
                folder={folder}
                purgeId={purgeId}
                restoreId={restoreId}
                onDelete={onDeleteFolder}
                onMove={onMoveFolder}
                onOpen={onOpenFolder}
                onPurge={onPurgeFolder}
                onRename={onRenameFolder}
                onRestore={onRestoreFolder}
              />
            ))}
            {visibleFiles.map((file) => (
              <DriveFileRow
                key={file.id}
                activeView={activeView}
                deleteId={deleteId}
                downloadId={downloadId}
                file={file}
                purgeId={purgeId}
                restoreId={restoreId}
                onDelete={onDeleteFile}
                onDownload={onDownload}
                onMove={onMoveFile}
                onPurge={onPurgeFile}
                onRename={onRenameFile}
                onRestore={onRestoreFile}
                onShare={onShareFile}
              />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  )
}
