import {
  Folder,
  FolderOpen,
  MoreHorizontal,
  MoveRight,
  Pencil,
  RotateCcw,
  Trash2,
} from "lucide-react"

import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Spinner } from "@/components/ui/spinner"
import { formatDriveDate } from "@/components/drive/format"
import type { DriveView } from "@/components/drive/types"
import type { FolderRecord } from "@/lib/api"

export function DriveFolderRow({
  activeView,
  deleteId,
  folder,
  purgeId,
  restoreId,
  onDelete,
  onMove,
  onOpen,
  onPurge,
  onRename,
  onRestore,
}: {
  activeView: DriveView
  deleteId: string | null
  folder: FolderRecord
  purgeId: string | null
  restoreId: string | null
  onDelete: (folderId: string) => void
  onMove: (folder: FolderRecord) => void
  onOpen: (folderId: string) => void
  onPurge: (folder: FolderRecord) => void
  onRename: (folder: FolderRecord) => void
  onRestore: (folderId: string) => void
}) {
  return (
    <div
      key={folder.id}
      data-cy="folder-item"
      className="flex items-center gap-3 rounded-lg border border-border p-3"
    >
      <button
        type="button"
        className="flex size-9 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground"
        onClick={() => {
          if (activeView === "my-drive") {
            onOpen(folder.id)
          }
        }}
        aria-label={`Open ${folder.name}`}
      >
        <Folder aria-hidden="true" />
      </button>
      <div className="min-w-0 flex-1">
        <button
          type="button"
          className="truncate font-medium"
          onClick={() => {
            if (activeView === "my-drive") {
              onOpen(folder.id)
            }
          }}
        >
          {folder.name}
        </button>
        <div className="flex flex-wrap gap-x-3 gap-y-1 text-xs text-muted-foreground">
          {activeView === "trash" ? (
            <span>Deleted {formatDriveDate(folder.deleted_at)}</span>
          ) : (
            <span>Created {formatDriveDate(folder.created_at)}</span>
          )}
        </div>
      </div>
      {activeView === "my-drive" ? (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              data-cy="folder-actions"
              variant="outline"
              size="icon"
              aria-label="Folder actions"
            >
              <MoreHorizontal />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-40">
            <DropdownMenuGroup>
              <DropdownMenuItem onSelect={() => onOpen(folder.id)}>
                <FolderOpen />
                Open
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => onRename(folder)}>
                <Pencil />
                Rename
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => onMove(folder)}>
                <MoveRight />
                Move
              </DropdownMenuItem>
            </DropdownMenuGroup>
            <DropdownMenuSeparator />
            <DropdownMenuGroup>
              <DropdownMenuItem
                variant="destructive"
                onSelect={() => onDelete(folder.id)}
                disabled={deleteId === folder.id}
              >
                <Trash2 />
                Delete
              </DropdownMenuItem>
            </DropdownMenuGroup>
          </DropdownMenuContent>
        </DropdownMenu>
      ) : (
        <div className="flex shrink-0 flex-wrap justify-end gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => onRestore(folder.id)}
            disabled={restoreId === folder.id || purgeId === folder.id}
          >
            {restoreId === folder.id ? (
              <Spinner data-icon="inline-start" />
            ) : (
              <RotateCcw data-icon="inline-start" />
            )}
            Restore
          </Button>
          <Button
            variant="destructive"
            size="sm"
            onClick={() => onPurge(folder)}
            disabled={purgeId === folder.id || restoreId === folder.id}
          >
            {purgeId === folder.id ? (
              <Spinner data-icon="inline-start" />
            ) : (
              <Trash2 data-icon="inline-start" />
            )}
            Force delete
          </Button>
        </div>
      )}
    </div>
  )
}
