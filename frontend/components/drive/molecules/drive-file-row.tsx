import {
  Download,
  MoveRight,
  Pencil,
  RotateCcw,
  Share2,
  Trash2,
} from "lucide-react"

import { DriveItemIcon } from "@/components/drive/atoms/drive-item-icon"
import { FileStateBadge } from "@/components/drive/atoms/file-state-badge"
import { formatDriveDate } from "@/components/drive/format"
import type { DriveView } from "@/components/drive/types"
import { Button } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import { formatBytes } from "@/lib/format"
import type { FileRecord, SharedFileRecord } from "@/lib/api"

function isSharedFile(
  file: FileRecord | SharedFileRecord
): file is SharedFileRecord {
  return "owner" in file
}

export function DriveFileRow({
  activeView,
  deleteId,
  downloadId,
  file,
  purgeId,
  restoreId,
  onDelete,
  onDownload,
  onMove,
  onPurge,
  onRename,
  onRestore,
  onShare,
}: {
  activeView: DriveView
  deleteId: string | null
  downloadId: string | null
  file: FileRecord | SharedFileRecord
  purgeId: string | null
  restoreId: string | null
  onDelete: (fileId: string) => void
  onDownload: (fileId: string) => void
  onMove: (file: FileRecord) => void
  onPurge: (file: FileRecord) => void
  onRename: (file: FileRecord) => void
  onRestore: (fileId: string) => void
  onShare: (file: FileRecord) => void
}) {
  return (
    <div
      key={file.id}
      data-cy="file-item"
      className="flex items-center gap-3 rounded-lg border border-border p-3"
    >
      <DriveItemIcon kind="file" />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate font-medium">{file.filename}</span>
          <FileStateBadge state={file.state} />
        </div>
        <div className="flex flex-wrap gap-x-3 gap-y-1 text-xs text-muted-foreground">
          <span>{formatBytes(file.size_bytes)}</span>
          {isSharedFile(file) ? (
            <span>
              Owner {file.owner.display_name} ({file.owner.email})
            </span>
          ) : activeView === "trash" ? (
            <span>Deleted {formatDriveDate(file.deleted_at)}</span>
          ) : (
            <span>Created {formatDriveDate(file.created_at)}</span>
          )}
          <span>Completed {formatDriveDate(file.completed_at)}</span>
        </div>
      </div>
      <div className="flex shrink-0 flex-wrap justify-end gap-2">
        {activeView !== "trash" && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => onDownload(file.id)}
            disabled={downloadId === file.id}
          >
            {downloadId === file.id ? (
              <Spinner data-icon="inline-start" />
            ) : (
              <Download data-icon="inline-start" />
            )}
            Download
          </Button>
        )}
        {activeView === "my-drive" && !isSharedFile(file) && (
          <>
            <Button
              variant="outline"
              size="sm"
              data-cy="file-actions"
              onClick={() => onRename(file)}
            >
              <Pencil data-icon="inline-start" />
              Rename
            </Button>
            <Button variant="outline" size="sm" onClick={() => onMove(file)}>
              <MoveRight data-icon="inline-start" />
              Move
            </Button>
            <Button
              variant="outline"
              size="sm"
              data-cy="share-action"
              onClick={() => onShare(file)}
            >
              <Share2 data-icon="inline-start" />
              Share
            </Button>
            <Button
              variant="destructive"
              size="sm"
              onClick={() => onDelete(file.id)}
              disabled={deleteId === file.id}
            >
              {deleteId === file.id ? (
                <Spinner data-icon="inline-start" />
              ) : (
                <Trash2 data-icon="inline-start" />
              )}
              Delete
            </Button>
          </>
        )}
        {activeView === "trash" && !isSharedFile(file) && (
          <>
            <Button
              variant="outline"
              size="sm"
              onClick={() => onRestore(file.id)}
              disabled={restoreId === file.id || purgeId === file.id}
            >
              {restoreId === file.id ? (
                <Spinner data-icon="inline-start" />
              ) : (
                <RotateCcw data-icon="inline-start" />
              )}
              Restore
            </Button>
            <Button
              variant="destructive"
              size="sm"
              onClick={() => onPurge(file)}
              disabled={purgeId === file.id || restoreId === file.id}
            >
              {purgeId === file.id ? (
                <Spinner data-icon="inline-start" />
              ) : (
                <Trash2 data-icon="inline-start" />
              )}
              Force delete
            </Button>
          </>
        )}
      </div>
    </div>
  )
}
