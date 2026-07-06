"use client"

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type FormEvent,
} from "react"
import { useRouter } from "next/navigation"
import {
  AlertCircle,
  Download,
  FileIcon,
  FolderOpen,
  Inbox,
  RotateCcw,
  Search,
  Share2,
  Trash2,
  Upload,
} from "lucide-react"

import { AppSidebar, type DriveView } from "@/components/app-sidebar"
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
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty"
import {
  Field,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { Kbd, KbdGroup } from "@/components/ui/kbd"
import { Progress } from "@/components/ui/progress"
import { Separator } from "@/components/ui/separator"
import { SidebarInset, SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar"
import { Spinner } from "@/components/ui/spinner"
import {
  api,
  uploadFileDirect,
  type FileRecord,
  type Share,
  type SharedFileRecord,
  type User,
} from "@/lib/api"
import { useAuth } from "@/lib/auth"
import { formatBytes } from "@/lib/format"
import { useModifierSymbol } from "@/lib/platform"
import {
  getApiErrorMessage,
  getShareErrorMessage,
} from "@/lib/shareErrors"

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

const viewCopy: Record<
  DriveView,
  {
    title: string
    description: string
    cardDescription: string
    cardTitle: string
    emptyTitle: string
    emptyDescription: string
  }
> = {
  "my-drive": {
    title: "My Drive",
    description: "Files you own and can share.",
    cardDescription: "Object storage",
    cardTitle: "Files",
    emptyTitle: "No files yet",
    emptyDescription: "Upload a file to store it in your drive.",
  },
  "shared-with-me": {
    title: "Shared with me",
    description: "Files other users shared with your account.",
    cardDescription: "Shared access",
    cardTitle: "Shared files",
    emptyTitle: "Nothing shared yet",
    emptyDescription: "Files shared with you will appear here.",
  },
  trash: {
    title: "Trash",
    description: "Deleted files you can restore.",
    cardDescription: "Deleted files",
    cardTitle: "Trash",
    emptyTitle: "Trash is empty",
    emptyDescription: "Deleted files will appear here until they are restored.",
  },
}

function formatDate(value: string | null): string {
  if (!value) {
    return "Pending"
  }
  return dateFormatter.format(new Date(value))
}

function isSharedFile(file: FileRecord | SharedFileRecord): file is SharedFileRecord {
  return "owner" in file
}

function ShareDialog({
  file,
  token,
  open,
  onOpenChange,
}: {
  file: FileRecord | null
  token: string | null
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const [shares, setShares] = useState<Share[]>([])
  const [email, setEmail] = useState("")
  const [loading, setLoading] = useState(false)
  const [submitting, setSubmitting] = useState(false)
  const [revokeId, setRevokeId] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const loadShares = useCallback(async () => {
    if (!token || !file) {
      setShares([])
      return
    }

    setLoading(true)
    setError(null)
    try {
      const response = await api.listShares(token, file.id)
      setShares(response.shares)
    } catch (caught) {
      setError(getApiErrorMessage(caught))
    } finally {
      setLoading(false)
    }
  }, [file, token])

  useEffect(() => {
    if (!open) {
      return
    }

    const timer = window.setTimeout(() => {
      void loadShares()
    }, 0)

    return () => {
      window.clearTimeout(timer)
    }
  }, [loadShares, open])

  function handleOpenChange(nextOpen: boolean) {
    if (!nextOpen) {
      setEmail("")
      setError(null)
      setShares([])
    }
    onOpenChange(nextOpen)
  }

  async function handleShare(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!token || !file) {
      setError("unauthorized: Your session expired. Log in again to share files.")
      return
    }

    setSubmitting(true)
    setError(null)
    try {
      const shareEmail = email.trim()
      await api.shareFile(token, file.id, shareEmail)
      setEmail("")
      await loadShares()
    } catch (caught) {
      setError(getShareErrorMessage(caught, email.trim()))
    } finally {
      setSubmitting(false)
    }
  }

  async function handleRevoke(granteeId: string) {
    if (!token || !file) {
      setError("unauthorized: Your session expired. Log in again to revoke access.")
      return
    }

    setRevokeId(granteeId)
    setError(null)
    try {
      await api.revokeShare(token, file.id, granteeId)
      await loadShares()
    } catch (caught) {
      setError(getApiErrorMessage(caught))
    } finally {
      setRevokeId(null)
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Share file</DialogTitle>
          <DialogDescription className="truncate">
            {file ? file.filename : "Select a file to manage sharing."}
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleShare}>
          <FieldGroup>
            <Field data-invalid={Boolean(error)}>
              <FieldLabel htmlFor="share-email">Grant access by email</FieldLabel>
              <div className="flex gap-2">
                <Input
                  id="share-email"
                  type="email"
                  value={email}
                  onChange={(event) => setEmail(event.target.value)}
                  placeholder="friend@example.com"
                  aria-invalid={Boolean(error)}
                  disabled={submitting}
                  required
                />
                <Button type="submit" disabled={submitting || !email.trim()}>
                  {submitting && <Spinner data-icon="inline-start" />}
                  Share
                </Button>
              </div>
              {error && <FieldError>{error}</FieldError>}
            </Field>
          </FieldGroup>
        </form>

        <div className="flex flex-col gap-2">
          <div className="text-sm font-medium">Current grantees</div>
          {loading ? (
            <div className="text-muted-foreground flex items-center gap-2 text-sm">
              <Spinner />
              Loading shares…
            </div>
          ) : shares.length === 0 ? (
            <p className="text-muted-foreground text-sm">
              This file is not shared with anyone.
            </p>
          ) : (
            shares.map((share) => (
              <div
                key={share.grantee.id}
                className="border-border flex items-center gap-3 rounded-lg border p-3"
              >
                <div className="min-w-0 flex-1">
                  <div className="truncate font-medium">
                    {share.grantee.display_name}
                  </div>
                  <div className="text-muted-foreground truncate text-xs">
                    {share.grantee.email}
                  </div>
                </div>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => void handleRevoke(share.grantee.id)}
                  disabled={revokeId === share.grantee.id}
                >
                  {revokeId === share.grantee.id && (
                    <Spinner data-icon="inline-start" />
                  )}
                  Revoke
                </Button>
              </div>
            ))
          )}
        </div>

        <DialogFooter showCloseButton />
      </DialogContent>
    </Dialog>
  )
}

export function DriveShell() {
  const { user, token, logout, refreshUser } = useAuth()
  const router = useRouter()
  const inputRef = useRef<HTMLInputElement>(null)
  const [activeView, setActiveView] = useState<DriveView>("my-drive")
  const [files, setFiles] = useState<FileRecord[]>([])
  const [trashFiles, setTrashFiles] = useState<FileRecord[]>([])
  const [sharedFiles, setSharedFiles] = useState<SharedFileRecord[]>([])
  const [loadingFiles, setLoadingFiles] = useState(true)
  const [uploadingName, setUploadingName] = useState<string | null>(null)
  const [uploadProgress, setUploadProgress] = useState(0)
  const [downloadId, setDownloadId] = useState<string | null>(null)
  const [deleteId, setDeleteId] = useState<string | null>(null)
  const [restoreId, setRestoreId] = useState<string | null>(null)
  const [shareFile, setShareFile] = useState<FileRecord | null>(null)
  const [error, setError] = useState<string | null>(null)

  const completedFiles = useMemo(
    () => files.filter((file) => file.state === "complete"),
    [files],
  )
  const completedTrashFiles = useMemo(
    () => trashFiles.filter((file) => file.state === "complete"),
    [trashFiles],
  )
  const completedSharedFiles = useMemo(
    () => sharedFiles.filter((file) => file.state === "complete"),
    [sharedFiles],
  )

  const visibleFiles = useMemo(() => {
    if (activeView === "trash") {
      return completedTrashFiles
    }
    if (activeView === "shared-with-me") {
      return completedSharedFiles
    }
    return completedFiles
  }, [activeView, completedFiles, completedSharedFiles, completedTrashFiles])

  const copy = viewCopy[activeView]

  const loadActiveView = useCallback(async () => {
    if (!token) {
      setLoadingFiles(false)
      return
    }

    setLoadingFiles(true)
    setError(null)
    try {
      if (activeView === "trash") {
        const response = await api.listTrash(token)
        setTrashFiles(response.files)
      } else if (activeView === "shared-with-me") {
        const response = await api.listSharedWithMe(token)
        setSharedFiles(response.files)
      } else {
        const response = await api.listFiles(token)
        setFiles(response.files)
      }
    } catch (caught) {
      setError(getApiErrorMessage(caught))
    } finally {
      setLoadingFiles(false)
    }
  }, [activeView, token])

  useEffect(() => {
    const timer = window.setTimeout(() => {
      void loadActiveView()
    }, 0)

    return () => {
      window.clearTimeout(timer)
    }
  }, [loadActiveView])

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
    setActiveView("my-drive")
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
      const response = await api.listFiles(token)
      setFiles(response.files)
    } catch (caught) {
      setError(getApiErrorMessage(caught))
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
      setError(getApiErrorMessage(caught))
    } finally {
      setDownloadId(null)
    }
  }

  async function handleDelete(fileId: string) {
    if (!token) {
      setError("Your session expired. Log in again to delete files.")
      return
    }

    setError(null)
    setDeleteId(fileId)
    try {
      await api.deleteFile(token, fileId)
      await refreshUser()
      await loadActiveView()
    } catch (caught) {
      setError(getApiErrorMessage(caught))
    } finally {
      setDeleteId(null)
    }
  }

  async function handleRestore(fileId: string) {
    if (!token) {
      setError("Your session expired. Log in again to restore files.")
      return
    }

    setError(null)
    setRestoreId(fileId)
    try {
      await api.restoreFile(token, fileId)
      await refreshUser()
      await loadActiveView()
    } catch (caught) {
      setError(getApiErrorMessage(caught))
    } finally {
      setRestoreId(null)
    }
  }

  return (
    <SidebarProvider>
      <CommandMenuProvider>
        <AppSidebar activeView={activeView} onViewChange={setActiveView} />
        <SidebarInset>
          <DriveHeader user={user} onLogout={handleLogout} />

          <main className="flex flex-col gap-5 p-6 md:p-7">
            <div>
              <h1 className="font-heading text-3xl font-bold tracking-tight">
                {copy.title}
              </h1>
              <p className="text-muted-foreground">{copy.description}</p>
            </div>

            <div className="grid gap-4 sm:grid-cols-3">
              <Card>
                <CardHeader>
                  <CardDescription>My files</CardDescription>
                  <CardTitle className="font-heading text-2xl">
                    {completedFiles.length}
                  </CardTitle>
                </CardHeader>
              </Card>
              <Card>
                <CardHeader>
                  <CardDescription>Shared with me</CardDescription>
                  <CardTitle className="font-heading text-2xl">
                    {completedSharedFiles.length}
                  </CardTitle>
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
                <CardDescription>{copy.cardDescription}</CardDescription>
                <CardTitle>{copy.cardTitle}</CardTitle>
                <CardAction className="flex gap-2">
                  <Button
                    variant="outline"
                    onClick={loadActiveView}
                    disabled={loadingFiles || Boolean(uploadingName)}
                  >
                    {loadingFiles && <Spinner data-icon="inline-start" />}
                    Reload
                  </Button>
                  {activeView === "my-drive" && (
                    <Button
                      onClick={() => inputRef.current?.click()}
                      disabled={Boolean(uploadingName)}
                    >
                      <Upload data-icon="inline-start" />
                      Upload
                    </Button>
                  )}
                </CardAction>
              </CardHeader>
              <CardContent>
                {loadingFiles ? (
                  <div className="text-muted-foreground flex items-center gap-2 text-sm">
                    <Spinner />
                    Loading files…
                  </div>
                ) : visibleFiles.length === 0 ? (
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
                        <Button onClick={() => inputRef.current?.click()}>
                          <Upload data-icon="inline-start" />
                          Upload file
                        </Button>
                      </EmptyContent>
                    )}
                  </Empty>
                ) : (
                  <div className="flex flex-col gap-2">
                    {visibleFiles.map((file) => (
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
                            {isSharedFile(file) ? (
                              <span>
                                Owner {file.owner.display_name} ({file.owner.email})
                              </span>
                            ) : activeView === "trash" ? (
                              <span>Deleted {formatDate(file.deleted_at)}</span>
                            ) : (
                              <span>Created {formatDate(file.created_at)}</span>
                            )}
                            <span>Completed {formatDate(file.completed_at)}</span>
                          </div>
                        </div>
                        <div className="flex shrink-0 flex-wrap justify-end gap-2">
                          {activeView !== "trash" && (
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
                          )}
                          {activeView === "my-drive" && (
                            <>
                              <Button
                                variant="outline"
                                size="sm"
                                onClick={() => setShareFile(file)}
                              >
                                <Share2 data-icon="inline-start" />
                                Share
                              </Button>
                              <Button
                                variant="destructive"
                                size="sm"
                                onClick={() => void handleDelete(file.id)}
                                disabled={deleteId === file.id}
                              >
                                {deleteId === file.id ? (
                                  <Spinner data-icon="inline-start" />
                                ) : (
                                  <Trash2 data-icon="inline-start" />
                                )}
                                Delete
                              </Button>
                            </>
                          )}
                          {activeView === "trash" && (
                            <Button
                              variant="outline"
                              size="sm"
                              onClick={() => void handleRestore(file.id)}
                              disabled={restoreId === file.id}
                            >
                              {restoreId === file.id ? (
                                <Spinner data-icon="inline-start" />
                              ) : (
                                <RotateCcw data-icon="inline-start" />
                              )}
                              Restore
                            </Button>
                          )}
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </CardContent>
            </Card>
          </main>

          <ShareDialog
            file={shareFile}
            token={token}
            open={Boolean(shareFile)}
            onOpenChange={(open) => {
              if (!open) {
                setShareFile(null)
              }
            }}
          />
        </SidebarInset>
      </CommandMenuProvider>
    </SidebarProvider>
  )
}
