import { FolderOpen, Inbox, Trash2, Upload } from "lucide-react"

import { Button } from "@/components/ui/button"
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty"
import type { viewCopy } from "@/components/drive/constants"
import type { DriveView } from "@/components/drive/types"

export function DriveEmptyState({
  activeView,
  copy,
  onUpload,
}: {
  activeView: DriveView
  copy: (typeof viewCopy)[DriveView]
  onUpload: () => void
}) {
  return (
    <Empty>
      <EmptyHeader>
        <EmptyMedia variant="icon">
          {activeView === "trash" ? (
            <Trash2 />
          ) : activeView === "shared-with-me" ? (
            <Inbox />
          ) : (
            <FolderOpen />
          )}
        </EmptyMedia>
        <EmptyTitle>{copy.emptyTitle}</EmptyTitle>
        <EmptyDescription>{copy.emptyDescription}</EmptyDescription>
      </EmptyHeader>
      {activeView === "my-drive" && (
        <EmptyContent>
          <Button data-cy="drive-upload-button" onClick={onUpload}>
            <Upload data-icon="inline-start" />
            Upload file
          </Button>
        </EmptyContent>
      )}
    </Empty>
  )
}
