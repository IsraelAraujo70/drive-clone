import { FolderPlus, Upload } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import type { DriveView } from "@/components/drive/types"

export function DriveToolbar({
  activeView,
  loadingFiles,
  uploadingName,
  onCreateFolder,
  onReload,
  onUpload,
}: {
  activeView: DriveView
  loadingFiles: boolean
  uploadingName: string | null
  onCreateFolder: () => void
  onReload: () => void
  onUpload: () => void
}) {
  return (
    <div className="flex gap-2">
      <Button
        variant="outline"
        onClick={onReload}
        disabled={loadingFiles || Boolean(uploadingName)}
      >
        {loadingFiles && <Spinner data-icon="inline-start" />}
        Reload
      </Button>
      {activeView === "my-drive" && (
        <>
          <Button
            variant="outline"
            data-cy="create-folder-button"
            onClick={onCreateFolder}
            disabled={Boolean(uploadingName)}
          >
            <FolderPlus data-icon="inline-start" />
            New folder
          </Button>
          <Button
            data-cy="drive-upload-button"
            onClick={onUpload}
            disabled={Boolean(uploadingName)}
          >
            <Upload data-icon="inline-start" />
            Upload
          </Button>
        </>
      )}
    </div>
  )
}
