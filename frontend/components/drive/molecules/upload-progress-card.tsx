import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Progress } from "@/components/ui/progress"

export function UploadProgressCard({
  uploadingName,
  uploadProgress,
  uploadParts,
  resumedFromPart,
}: {
  uploadingName: string
  uploadProgress: number
  uploadParts: { done: number; total: number } | null
  resumedFromPart: number | null
}) {
  return (
    <Card size="sm" data-cy="upload-progress">
      <CardHeader>
        <CardDescription>Uploading</CardDescription>
        <CardTitle className="truncate">{uploadingName}</CardTitle>
        <CardAction>
          <Badge variant="secondary">{uploadProgress}%</Badge>
        </CardAction>
      </CardHeader>
      <CardContent className="flex flex-col gap-2">
        <Progress value={uploadProgress} aria-label="Upload progress" />
        {uploadParts && uploadParts.total > 0 && (
          <p className="text-sm text-muted-foreground">
            Part {Math.min(uploadParts.done + 1, uploadParts.total)} of{" "}
            {uploadParts.total}
            {resumedFromPart !== null &&
              ` — resumed from part ${resumedFromPart}`}
          </p>
        )}
      </CardContent>
    </Card>
  )
}
