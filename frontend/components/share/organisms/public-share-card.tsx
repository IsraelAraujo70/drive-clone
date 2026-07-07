import { Download, FileIcon } from "lucide-react"

import { Button } from "@/components/ui/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Spinner } from "@/components/ui/spinner"
import { formatBytes } from "@/lib/format"
import type { PublicShareLinkState } from "@/lib/shareLink"

type LoadState = { status: "loading" } | PublicShareLinkState

export function PublicShareCard({ state }: { state: LoadState }) {
  return (
    <Card className="w-full max-w-md">
      {state.status === "loading" && (
        <CardContent className="flex items-center gap-2 py-10 text-sm text-muted-foreground">
          <Spinner />
          Loading shared file…
        </CardContent>
      )}

      {state.status === "unavailable" && (
        <>
          <CardHeader>
            <CardTitle>Link unavailable</CardTitle>
            <CardDescription>{state.message}</CardDescription>
          </CardHeader>
          <CardContent className="text-sm text-muted-foreground">
            Ask the person who shared it for a fresh link.
          </CardContent>
        </>
      )}

      {state.status === "ready" && (
        <>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <FileIcon className="size-5 shrink-0" aria-hidden />
              <span className="truncate">{state.file.filename}</span>
            </CardTitle>
            <CardDescription>
              {formatBytes(state.file.size_bytes)} · {state.file.content_type}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Button asChild className="w-full">
              <a href={state.file.download_url} download={state.file.filename}>
                <Download data-icon="inline-start" />
                Download
              </a>
            </Button>
          </CardContent>
        </>
      )}
    </Card>
  )
}
