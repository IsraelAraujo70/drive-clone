"use client"

import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { useRouter } from "next/navigation"
import { AlertCircle, Download, FileIcon, FolderOpen, Search, Upload } from "lucide-react"

import { AppSidebar } from "@/components/app-sidebar"
import { CommandMenuProvider, useCommandMenu } from "@/components/command-menu"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty"
import { Kbd, KbdGroup } from "@/components/ui/kbd"
import { Progress } from "@/components/ui/progress"
import { Separator } from "@/components/ui/separator"
import { SidebarInset, SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar"
import { Spinner } from "@/components/ui/spinner"
import { api, uploadFileDirect, type FileRecord, type User } from "@/lib/api"
import { useAuth } from "@/lib/auth"
import { formatBytes } from "@/lib/format"
import { useModifierSymbol } from "@/lib/platform"

function HeaderSearch() {
  const { openMenu } = useCommandMenu()
  const modifier = useModifierSymbol()

  return (
    <button
      type="button"
      onClick={openMenu}
      className="text-muted-foreground border-input bg-background hover:bg-accent/40 flex w-full max-w-md items-center gap-2 rounded-lg border px-3 py-2 text-sm transition-colors"
    >
      <Search aria-hidden="true" className="size-4" />
      <span className="flex-1 text-left">Search files</span>
      <KbdGroup>
        <Kbd>{modifier}</Kbd>
        <Kbd>K</Kbd>
      </KbdGroup>
    </button>
  )
}

function DriveHeader({ user, onLogout }: { user: User; onLogout: () => void }) {
  return (
    <header className="border-border bg-card/80 sticky top-0 z-10 flex items-center gap-3 border-b px-4 py-3 backdrop-blur">
      <SidebarTrigger aria-label="Toggle sidebar (⌘B)" />
      <Separator orientation="vertical" className="mr-1 !h-6" />
      <HeaderSearch />
      <div className="ml-auto flex items-center gap-2">
        <Badge variant="outline" title={user.email}>
          {user.display_name}
        </Badge>
        <Button variant="outline" onClick={onLogout}>
          Log out
        </Button>
      </div>
    </header>
  )
}

const dateFormatter = new Intl.DateTimeFormat(undefined, {
  dateStyle: "medium",
  timeStyle: "short",
})

function formatDate(value: string | null): string {
  if (!value) {
    return "Pending"
  }
  return dateFormatter.format(new Date(value))
}

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "Something went wrong. Try again."
}

export function DriveShell() {
  const { user, token, logout, refreshUser } = useAuth()
  const router = useRouter()
  const inputRef = useRef<HTMLInputElement>(null)
  const [files, setFiles] = useState<FileRecord[]>([])
  const [loadingFiles, setLoadingFiles] = useState(true)
  const [uploadingName, setUploadingName] = useState<string | null>(null)
  const [uploadProgress, setUploadProgress] = useState(0)
  const [downloadId, setDownloadId] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const completedFiles = useMemo(
    () => files.filter((file) => file.state === "complete"),
    [files],
  )

  const loadFiles = useCallback(async () => {
    if (!token) {
      setLoadingFiles(false)
      return
    }

    setLoadingFiles(true)
    setError(null)
    try {
      const response = await api.listFiles(token)
      setFiles(response.files)
    } catch (caught) {
      setError(getErrorMessage(caught))
    } finally {
      setLoadingFiles(false)
    }
  }, [token])

  useEffect(() => {
    const timer = window.setTimeout(() => {
      void loadFiles()
    }, 0)

    return () => {
      window.clearTimeout(timer)
    }
  }, [loadFiles])

  if (!user) {
    return null // RequireAuth guarantees a user; this satisfies the type checker
  }

  async function handleLogout() {
    await logout()
    router.replace("/")
  }

  async function handleUpload(file: File) {
    if (!token) {
      setError("Your session expired. Log in again to upload files.")
      return
    }

    setError(null)
    setUploadingName(file.name)
    setUploadProgress(0)

    try {
      const upload = await api.createUpload(token, {
        filename: file.name,
        content_type: file.type || "application/octet-stream",
        size_bytes: file.size,
        checksum_sha256: null,
      })

      await uploadFileDirect(upload.upload_url, file, (progress) => {
        setUploadProgress(progress.percent)
      })

      await api.completeUpload(token, upload.file_id)
      await refreshUser()
      await loadFiles()
    } catch (caught) {
      setError(getErrorMessage(caught))
    } finally {
      setUploadingName(null)
      setUploadProgress(0)
      if (inputRef.current) {
        inputRef.current.value = ""
      }
    }
  }

  async function handleDownload(fileId: string) {
    if (!token) {
      setError("Your session expired. Log in again to download files.")
      return
    }

    setError(null)
    setDownloadId(fileId)
    try {
      const response = await api.createDownload(token, fileId)
      window.location.assign(response.download_url)
    } catch (caught) {
      setError(getErrorMessage(caught))
    } finally {
      setDownloadId(null)
    }
  }

  return (
    <SidebarProvider>
      <CommandMenuProvider>
        <AppSidebar />
        <SidebarInset>
          <DriveHeader user={user} onLogout={handleLogout} />

          <main className="flex flex-col gap-5 p-6 md:p-7">
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
                  <CardTitle className="font-heading text-2xl">
                    {completedFiles.length}
                  </CardTitle>
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

            <input
              ref={inputRef}
              type="file"
              className="sr-only"
              onChange={(event) => {
                const file = event.target.files?.[0]
                if (file) {
                  void handleUpload(file)
                }
              }}
            />

            {error && (
              <Alert variant="destructive">
                <AlertCircle />
                <AlertTitle>File action failed</AlertTitle>
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}

            {uploadingName && (
              <Card size="sm">
                <CardHeader>
                  <CardDescription>Uploading</CardDescription>
                  <CardTitle className="truncate">{uploadingName}</CardTitle>
                  <CardAction>
                    <Badge variant="secondary">{uploadProgress}%</Badge>
                  </CardAction>
                </CardHeader>
                <CardContent>
                  <Progress value={uploadProgress} aria-label="Upload progress" />
                </CardContent>
              </Card>
            )}

            <Card>
              <CardHeader>
                <CardDescription>Object storage</CardDescription>
                <CardTitle>Files</CardTitle>
                <CardAction className="flex gap-2">
                  <Button
                    variant="outline"
                    onClick={loadFiles}
                    disabled={loadingFiles || Boolean(uploadingName)}
                  >
                    {loadingFiles && <Spinner data-icon="inline-start" />}
                    Reload
                  </Button>
                  <Button
                    onClick={() => inputRef.current?.click()}
                    disabled={Boolean(uploadingName)}
                  >
                    <Upload data-icon="inline-start" />
                    Upload
                  </Button>
                </CardAction>
              </CardHeader>
              <CardContent>
                {loadingFiles ? (
                  <div className="text-muted-foreground flex items-center gap-2 text-sm">
                    <Spinner />
                    Loading files…
                  </div>
                ) : completedFiles.length === 0 ? (
                  <Empty>
                    <EmptyHeader>
                      <EmptyMedia variant="icon">
                        <FolderOpen />
                      </EmptyMedia>
                      <EmptyTitle>No files yet</EmptyTitle>
                      <EmptyDescription>
                        Upload a file to store it in your drive.
                      </EmptyDescription>
                    </EmptyHeader>
                    <EmptyContent>
                      <Button onClick={() => inputRef.current?.click()}>
                        <Upload data-icon="inline-start" />
                        Upload file
                      </Button>
                    </EmptyContent>
                  </Empty>
                ) : (
                  <div className="flex flex-col gap-2">
                    {completedFiles.map((file) => (
                      <div
                        key={file.id}
                        className="border-border flex items-center gap-3 rounded-lg border p-3"
                      >
                        <div className="bg-muted text-muted-foreground flex size-9 shrink-0 items-center justify-center rounded-md">
                          <FileIcon aria-hidden="true" />
                        </div>
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center gap-2">
                            <span className="truncate font-medium">
                              {file.filename}
                            </span>
                            <Badge variant="secondary">{file.state}</Badge>
                          </div>
                          <div className="text-muted-foreground flex flex-wrap gap-x-3 gap-y-1 text-xs">
                            <span>{formatBytes(file.size_bytes)}</span>
                            <span>Created {formatDate(file.created_at)}</span>
                            <span>Completed {formatDate(file.completed_at)}</span>
                          </div>
                        </div>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => void handleDownload(file.id)}
                          disabled={downloadId === file.id}
                        >
                          {downloadId === file.id ? (
                            <Spinner data-icon="inline-start" />
                          ) : (
                            <Download data-icon="inline-start" />
                          )}
                          Download
                        </Button>
                      </div>
                    ))}
                  </div>
                )}
              </CardContent>
            </Card>
          </main>
        </SidebarInset>
      </CommandMenuProvider>
    </SidebarProvider>
  )
}
