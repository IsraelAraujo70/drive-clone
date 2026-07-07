import { Upload } from "lucide-react"

import { Button } from "@/components/ui/button"
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { formatDriveDate } from "@/components/drive/format"
import { formatBytes } from "@/lib/format"
import type { PendingResumableUpload } from "@/lib/resumableUploads"

export function PendingUploadCard({
  pendingUploads,
  clearingExpired,
  onClearExpired,
  onDismiss,
  onResume,
}: {
  pendingUploads: PendingResumableUpload[]
  clearingExpired: boolean
  onClearExpired: () => void
  onDismiss: (pending: PendingResumableUpload) => void
  onResume: (pending: PendingResumableUpload) => void
}) {
  return (
    <Card size="sm" data-cy="pending-upload-card">
      <CardHeader>
        <CardDescription>Interrupted uploads</CardDescription>
        <CardTitle>
          {pendingUploads.length === 1
            ? "1 upload can resume"
            : `${pendingUploads.length} uploads can resume`}
        </CardTitle>
        <CardAction>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            disabled={clearingExpired}
            onClick={onClearExpired}
          >
            {clearingExpired ? "Clearing…" : "Clear expired"}
          </Button>
        </CardAction>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <p className="text-sm text-muted-foreground">
          Select the same local file again to continue from the parts already
          saved.
        </p>
        <div className="flex flex-col gap-2">
          {pendingUploads.map((pending) => {
            const server = pending.server
            const partsTotal =
              server && server.part_size_bytes > 0
                ? Math.ceil(server.size_bytes / server.part_size_bytes)
                : null
            return (
              <div
                key={pending.key}
                className="flex flex-col gap-3 rounded-lg border border-border p-3 sm:flex-row sm:items-center"
              >
                <div className="min-w-0 flex-1">
                  <div className="truncate font-medium">
                    {pending.upload.filename}
                  </div>
                  <div className="text-sm text-muted-foreground">
                    {formatBytes(pending.upload.size_bytes)} expires{" "}
                    {formatDriveDate(pending.upload.expires_at)}
                    {server && partsTotal !== null && (
                      <>
                        {" · "}
                        {server.parts_received}/{partsTotal} parts saved
                      </>
                    )}
                  </div>
                </div>
                <div className="flex gap-2">
                  <Button
                    type="button"
                    data-cy="dismiss-upload"
                    variant="outline"
                    onClick={() => onDismiss(pending)}
                  >
                    Dismiss
                  </Button>
                  <Button
                    type="button"
                    data-cy="resume-upload"
                    onClick={() => onResume(pending)}
                  >
                    <Upload data-icon="inline-start" />
                    Resume
                  </Button>
                </div>
              </div>
            )
          })}
        </div>
      </CardContent>
    </Card>
  )
}
