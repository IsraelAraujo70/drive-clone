"use client"

import { useState } from "react"
import { Folder, FolderOpen } from "lucide-react"

import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { FieldError } from "@/components/ui/field"
import { Spinner } from "@/components/ui/spinner"
import type { DriveItem } from "@/components/drive/types"
import { isDescendantFolder } from "@/lib/driveTree"
import type { FolderRecord } from "@/lib/api"

export function MoveDialog({
  target,
  folders,
  submitting,
  error,
  onOpenChange,
  onSubmit,
}: {
  target: DriveItem | null
  folders: FolderRecord[]
  submitting: boolean
  error: string | null
  onOpenChange: (open: boolean) => void
  onSubmit: (parentFolderId: string | null) => void
}) {
  const [selected, setSelected] = useState<string | null | undefined>(undefined)
  const effectiveSelected =
    selected === undefined ? (target?.item.parent_folder_id ?? null) : selected

  function handleOpenChange(nextOpen: boolean) {
    if (!nextOpen) {
      setSelected(undefined)
    }
    onOpenChange(nextOpen)
  }

  return (
    <Dialog open={Boolean(target)} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Move {target?.kind ?? "item"}</DialogTitle>
          <DialogDescription>Select a destination folder.</DialogDescription>
        </DialogHeader>
        <div className="flex max-h-80 flex-col gap-2 overflow-y-auto">
          <Button
            type="button"
            variant={effectiveSelected === null ? "default" : "outline"}
            className="justify-start"
            onClick={() => setSelected(null)}
          >
            <FolderOpen data-icon="inline-start" />
            My Drive
          </Button>
          {folders.map((folder) => {
            const disabled =
              target?.kind === "folder" &&
              (folder.id === target.item.id ||
                isDescendantFolder(folders, target.item.id, folder.id))
            return (
              <Button
                key={folder.id}
                type="button"
                variant={
                  effectiveSelected === folder.id ? "default" : "outline"
                }
                className="justify-start"
                onClick={() => setSelected(folder.id)}
                disabled={disabled}
              >
                <Folder data-icon="inline-start" />
                <span className="truncate">{folder.name}</span>
              </Button>
            )
          })}
        </div>
        {error && <FieldError>{error}</FieldError>}
        <DialogFooter>
          <Button
            type="button"
            onClick={() => onSubmit(effectiveSelected)}
            disabled={submitting}
          >
            {submitting && <Spinner data-icon="inline-start" />}
            Move
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
