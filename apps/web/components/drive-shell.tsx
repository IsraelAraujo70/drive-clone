"use client"

import { useEffect, useState } from "react"
import { useRouter } from "next/navigation"
import { FolderOpen, HardDrive, Search, Share2, Trash2 } from "lucide-react"

import { Brand } from "@/components/brand"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty"
import { Progress } from "@/components/ui/progress"
import { api } from "@/lib/api"
import { useAuth } from "@/lib/auth"
import { formatBytes } from "@/lib/format"
import { formatHealthStatus } from "@/lib/health"

type ApiState =
  | { status: "loading"; label: string }
  | { status: "ok"; label: string }
  | { status: "error"; label: string }

export function DriveShell() {
  const { user, logout } = useAuth()
  const router = useRouter()
  const [apiState, setApiState] = useState<ApiState>({
    status: "loading",
    label: "Checking API",
  })

  useEffect(() => {
    let cancelled = false
    api
      .health()
      .then((health) => {
        if (!cancelled) {
          setApiState({ status: "ok", label: formatHealthStatus(health) })
        }
      })
      .catch(() => {
        if (!cancelled) {
          setApiState({ status: "error", label: "API offline" })
        }
      })
    return () => {
      cancelled = true
    }
  }, [])

  if (!user) {
    return null // RequireAuth guarantees a user; this satisfies the type checker
  }

  const usedPercent = Math.min(
    100,
    (user.storage_used_bytes / user.storage_quota_bytes) * 100,
  )

  async function handleLogout() {
    await logout()
    router.replace("/")
  }

  return (
    <div className="grid min-h-svh md:grid-cols-[260px_1fr]">
      <aside className="border-border bg-card flex flex-col gap-6 border-b p-6 md:border-r md:border-b-0">
        <Brand />

        <nav aria-label="Drive sections" className="flex flex-col gap-1">
          <Button variant="secondary" className="justify-start">
            <HardDrive data-icon="inline-start" />
            My Drive
          </Button>
          <Button variant="ghost" className="justify-start">
            <Share2 data-icon="inline-start" />
            Shared
          </Button>
          <Button variant="ghost" className="justify-start">
            <Trash2 data-icon="inline-start" />
            Trash
          </Button>
        </nav>

        <div className="mt-auto flex flex-col gap-2">
          <strong className="text-sm">
            {formatBytes(user.storage_used_bytes)} of{" "}
            {formatBytes(user.storage_quota_bytes)}
          </strong>
          <Progress value={usedPercent} aria-label="Storage usage" />
          <span className="text-muted-foreground text-xs">Storage used</span>
        </div>
      </aside>

      <div className="flex min-w-0 flex-col">
        <header className="border-border bg-card/80 flex flex-wrap items-center justify-between gap-3 border-b px-7 py-4">
          <div className="text-muted-foreground border-input bg-background flex w-full max-w-md items-center gap-2 rounded-lg border px-3 py-2 text-sm">
            <Search aria-hidden="true" className="size-4" />
            Search files
          </div>
          <div className="flex items-center gap-2">
            <Badge
              variant={apiState.status === "error" ? "destructive" : "secondary"}
            >
              {apiState.label}
            </Badge>
            <Badge variant="outline" title={user.email}>
              {user.display_name}
            </Badge>
            <Button variant="outline" onClick={handleLogout}>
              Log out
            </Button>
          </div>
        </header>

        <main className="flex flex-col gap-5 p-7">
          <div>
            <h1 className="font-heading text-3xl font-bold tracking-tight">
              My Drive
            </h1>
            <p className="text-muted-foreground">Signed in as {user.email}.</p>
          </div>

          <div className="grid gap-4 sm:grid-cols-3">
            <Card>
              <CardHeader>
                <CardDescription>Files</CardDescription>
                <CardTitle className="font-heading text-2xl">0</CardTitle>
              </CardHeader>
            </Card>
            <Card>
              <CardHeader>
                <CardDescription>Folders</CardDescription>
                <CardTitle className="font-heading text-2xl">0</CardTitle>
              </CardHeader>
            </Card>
            <Card>
              <CardHeader>
                <CardDescription>Storage used</CardDescription>
                <CardTitle className="font-heading text-2xl">
                  {formatBytes(user.storage_used_bytes)}
                </CardTitle>
              </CardHeader>
            </Card>
          </div>

          <Card className="py-0">
            <Empty>
              <EmptyHeader>
                <EmptyMedia variant="icon">
                  <FolderOpen />
                </EmptyMedia>
                <EmptyTitle>No files yet</EmptyTitle>
                <EmptyDescription>
                  Uploads arrive in the next milestone.
                </EmptyDescription>
              </EmptyHeader>
            </Empty>
          </Card>
        </main>
      </div>
    </div>
  )
}
