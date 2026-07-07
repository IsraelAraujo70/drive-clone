"use client"

import { use, useEffect, useState } from "react"
import { Download, FileIcon } from "lucide-react"

import { Brand } from "@/components/brand"
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
import { loadPublicShareLink, type PublicShareLinkState } from "@/lib/shareLink"

type LoadState = { status: "loading" } | PublicShareLinkState

export default function SharedFilePage({
  params,
}: {
  params: Promise<{ token: string }>
}) {
  const { token } = use(params)
  const [state, setState] = useState<LoadState>({ status: "loading" })

  useEffect(() => {
    let active = true
    void loadPublicShareLink(token).then((next) => {
      if (active) {
        setState(next)
      }
    })
    return () => {
      active = false
    }
  }, [token])

  return (
    <div className="flex min-h-svh flex-col items-center justify-center gap-8 p-6">
      <Brand />
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
    </div>
  )
}
