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
  ChevronRight,
  Download,
  FileIcon,
  Folder,
  FolderPlus,
  FolderOpen,
  Inbox,
  MoreHorizontal,
  MoveRight,
  Pencil,
  RotateCcw,
  Search,
  Share2,
  Trash2,
  Upload,
} from "lucide-react"

import { AppSidebar, type DriveView } from "@/components/app-sidebar"
import { CommandMenuProvider, useCommandMenu } from "@/components/command-menu"
import { ThemeToggleButton } from "@/components/theme-provider"
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
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
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
import {
  SidebarInset,
  SidebarProvider,
  SidebarTrigger,
} from "@/components/ui/sidebar"
import { Spinner } from "@/components/ui/spinner"
import {
  api,
  uploadFilePart,
  type FileRecord,
  type FolderPathEntry,
  type FolderRecord,
  type Share,
  type SharedFileRecord,
  type User,
} from "@/lib/api"
import { useAuth } from "@/lib/auth"
import { formatBytes } from "@/lib/format"
import { useModifierSymbol } from "@/lib/platform"
import {
  clearExpiredUploads,
  fileMatchesStoredUpload,
  forgetUpload,
  mergePendingUploads,
  pendingStoredUploads,
  readStoredUploads,
  rememberUpload,
  resumableUploadKey,
  resumedProgress,
  type PendingResumableUpload,
  type ServerPendingUpload,
} from "@/lib/resumableUploads"
import { getApiErrorMessage, getShareErrorMessage } from "@/lib/shareErrors"
import { getUploadErrorMessage } from "@/lib/uploadErrors"

function HeaderSearch() {
  const { openMenu } = useCommandMenu()
  const modifier = useModifierSymbol()

  return (
    <button
      type="button"
      onClick={openMenu}
      className="flex w-full max-w-md items-center gap-2 rounded-lg border border-input bg-background px-3 py-2 text-sm text-muted-foreground transition-colors hover:bg-accent/40"
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
    <header className="sticky top-0 z-10 flex items-center gap-3 border-b border-border bg-card/80 px-4 py-3 backdrop-blur">
      <SidebarTrigger aria-label="Toggle sidebar (⌘B)" />
      <Separator orientation="vertical" className="mr-1 !h-6" />
      <HeaderSearch />
      <div className="ml-auto flex items-center gap-2">
        <ThemeToggleButton />
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

function isSharedFile(
  file: FileRecord | SharedFileRecord
): file is SharedFileRecord {
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
      setError(
        "unauthorized: Your session expired. Log in again to share files."
      )
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
      setError(
        "unauthorized: Your session expired. Log in again to revoke access."
      )
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
              <FieldLabel htmlFor="share-email">
                Grant access by email
              </FieldLabel>
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
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Spinner />
              Loading shares…
            </div>
          ) : shares.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              This file is not shared with anyone.
            </p>
          ) : (
            shares.map((share) => (
              <div
                key={share.grantee.id}
                className="flex items-center gap-3 rounded-lg border border-border p-3"
              >
                <div className="min-w-0 flex-1">
                  <div className="truncate font-medium">
                    {share.grantee.display_name}
                  </div>
                  <div className="truncate text-xs text-muted-foreground">
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

type DriveItem =
  { kind: "file"; item: FileRecord } | { kind: "folder"; item: FolderRecord }

function isDescendantFolder(
  folders: FolderRecord[],
  folderId: string,
  candidateParentId: string
): boolean {
  const byParent = new Map<string | null, FolderRecord[]>()
  for (const folder of folders) {
    const siblings = byParent.get(folder.parent_folder_id) ?? []
    siblings.push(folder)
    byParent.set(folder.parent_folder_id, siblings)
  }

  const stack = [...(byParent.get(folderId) ?? [])]
  while (stack.length > 0) {
    const folder = stack.pop()
    if (!folder) {
      continue
    }
    if (folder.id === candidateParentId) {
      return true
    }
    stack.push(...(byParent.get(folder.id) ?? []))
  }
  return false
}

function CreateFolderDialog({
  open,
  submitting,
  error,
  onOpenChange,
  onSubmit,
}: {
  open: boolean
  submitting: boolean
  error: string | null
  onOpenChange: (open: boolean) => void
  onSubmit: (name: string) => void
}) {
  const [name, setName] = useState("")

  function handleOpenChange(nextOpen: boolean) {
    if (!nextOpen) {
      setName("")
    }
    onOpenChange(nextOpen)
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Create folder</DialogTitle>
          <DialogDescription>
            Add a folder in the current location.
          </DialogDescription>
        </DialogHeader>
        <form
          onSubmit={(event) => {
            event.preventDefault()
            onSubmit(name)
          }}
        >
          <FieldGroup>
            <Field data-invalid={Boolean(error)}>
              <FieldLabel htmlFor="folder-name">Folder name</FieldLabel>
              <Input
                id="folder-name"
                value={name}
                onChange={(event) => setName(event.target.value)}
                aria-invalid={Boolean(error)}
                disabled={submitting}
                required
              />
              {error && <FieldError>{error}</FieldError>}
            </Field>
          </FieldGroup>
          <DialogFooter>
            <Button type="submit" disabled={submitting || !name.trim()}>
              {submitting && <Spinner data-icon="inline-start" />}
              Create
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

function RenameDialog({
  target,
  submitting,
  error,
  onOpenChange,
  onSubmit,
}: {
  target: DriveItem | null
  submitting: boolean
  error: string | null
  onOpenChange: (open: boolean) => void
  onSubmit: (name: string) => void
}) {
  const defaultName =
    target?.kind === "file" ? target.item.filename : (target?.item.name ?? "")

  return (
    <Dialog open={Boolean(target)} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md" key={target?.item.id ?? "rename"}>
        <DialogHeader>
          <DialogTitle>Rename {target?.kind ?? "item"}</DialogTitle>
          <DialogDescription>
            Update the visible name in your drive.
          </DialogDescription>
        </DialogHeader>
        <form
          onSubmit={(event) => {
            event.preventDefault()
            const form = event.currentTarget
            const data = new FormData(form)
            onSubmit(String(data.get("name") ?? ""))
          }}
        >
          <FieldGroup>
            <Field data-invalid={Boolean(error)}>
              <FieldLabel htmlFor="rename-name">Name</FieldLabel>
              <Input
                id="rename-name"
                name="name"
                defaultValue={defaultName}
                aria-invalid={Boolean(error)}
                disabled={submitting}
                required
              />
              {error && <FieldError>{error}</FieldError>}
            </Field>
          </FieldGroup>
          <DialogFooter>
            <Button type="submit" disabled={submitting}>
              {submitting && <Spinner data-icon="inline-start" />}
              Rename
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

function MoveDialog({
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

export function DriveShell() {
  const { user, token, logout, refreshUser } = useAuth()
  const router = useRouter()
  const inputRef = useRef<HTMLInputElement>(null)
  const resumeInputRef = useRef<HTMLInputElement>(null)
  const [activeView, setActiveView] = useState<DriveView>("my-drive")
  const [currentFolderId, setCurrentFolderId] = useState<string | null>(null)
  const [breadcrumbs, setBreadcrumbs] = useState<FolderPathEntry[]>([])
  const [folders, setFolders] = useState<FolderRecord[]>([])
  const [allFolders, setAllFolders] = useState<FolderRecord[]>([])
  const [files, setFiles] = useState<FileRecord[]>([])
  const [trashFolders, setTrashFolders] = useState<FolderRecord[]>([])
  const [trashFiles, setTrashFiles] = useState<FileRecord[]>([])
  const [sharedFiles, setSharedFiles] = useState<SharedFileRecord[]>([])
  const [loadingFiles, setLoadingFiles] = useState(true)
  const [uploadingName, setUploadingName] = useState<string | null>(null)
  const [uploadProgress, setUploadProgress] = useState(0)
  const [uploadParts, setUploadParts] = useState<{
    done: number
    total: number
  } | null>(null)
  const [resumedFromPart, setResumedFromPart] = useState<number | null>(null)
  const [pendingUploads, setPendingUploads] = useState<
    PendingResumableUpload[]
  >([])
  const [clearingExpired, setClearingExpired] = useState(false)
  const [resumeTarget, setResumeTarget] =
    useState<PendingResumableUpload | null>(null)
  const [downloadId, setDownloadId] = useState<string | null>(null)
  const [deleteId, setDeleteId] = useState<string | null>(null)
  const [restoreId, setRestoreId] = useState<string | null>(null)
  const [shareFile, setShareFile] = useState<FileRecord | null>(null)
  const [createFolderOpen, setCreateFolderOpen] = useState(false)
  const [renameTarget, setRenameTarget] = useState<DriveItem | null>(null)
  const [moveTarget, setMoveTarget] = useState<DriveItem | null>(null)
  const [dialogSubmitting, setDialogSubmitting] = useState(false)
  const [dialogError, setDialogError] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const completedFiles = useMemo(
    () => files.filter((file) => file.state === "complete"),
    [files]
  )
  const completedTrashFiles = useMemo(
    () => trashFiles.filter((file) => file.state === "complete"),
    [trashFiles]
  )
  const completedSharedFiles = useMemo(
    () => sharedFiles.filter((file) => file.state === "complete"),
    [sharedFiles]
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

  const visibleFolders = useMemo(() => {
    if (activeView === "trash") {
      return trashFolders
    }
    if (activeView === "my-drive") {
      return folders
    }
    return []
  }, [activeView, folders, trashFolders])

  const copy = viewCopy[activeView]

  // Fast, localStorage-only refresh used while an upload is in flight.
  const refreshPendingUploads = useCallback(() => {
    setPendingUploads(pendingStoredUploads())
  }, [])

  // Authoritative refresh: the server owns which sessions still exist and when
  // they expire; localStorage only supplies the local-file key. Falls back to
  // localStorage when the server is unreachable.
  const syncPendingUploads = useCallback(async () => {
    if (!token) {
      setPendingUploads([])
      return
    }
    try {
      const response = await api.listPendingUploads(token)
      const serverUploads: ServerPendingUpload[] = response.uploads
      setPendingUploads(mergePendingUploads(serverUploads))
    } catch {
      setPendingUploads(pendingStoredUploads())
    }
  }, [token])

  const handleClearExpiredUploads = useCallback(async () => {
    setClearingExpired(true)
    try {
      let activeFileIds: Set<string> | null = null
      if (token) {
        try {
          const response = await api.listPendingUploads(token)
          activeFileIds = new Set(
            response.uploads.map((upload) => upload.file_id)
          )
        } catch {
          activeFileIds = null
        }
      }
      clearExpiredUploads(activeFileIds)
      await syncPendingUploads()
    } finally {
      setClearingExpired(false)
    }
  }, [syncPendingUploads, token])

  const loadActiveView = useCallback(async () => {
    if (!token) {
      setLoadingFiles(false)
      return
    }

    setLoadingFiles(true)
    setError(null)
    try {
      if (activeView === "trash") {
        const response = await api.listDriveTrash(token)
        setTrashFolders(response.folders)
        setTrashFiles(response.files)
      } else if (activeView === "shared-with-me") {
        const response = await api.listSharedWithMe(token)
        setSharedFiles(response.files)
      } else {
        const response = await api.browseDrive(token, currentFolderId)
        setFolders(response.folders)
        setFiles(response.files)
        setBreadcrumbs(response.breadcrumbs)
      }
    } catch (caught) {
      setError(getApiErrorMessage(caught))
    } finally {
      setLoadingFiles(false)
    }
  }, [activeView, currentFolderId, token])

  useEffect(() => {
    const timer = window.setTimeout(() => {
      void loadActiveView()
    }, 0)

    return () => {
      window.clearTimeout(timer)
    }
  }, [loadActiveView])

  useEffect(() => {
    const timer = window.setTimeout(() => {
      void syncPendingUploads()
    }, 0)
    return () => {
      window.clearTimeout(timer)
    }
  }, [syncPendingUploads])

  if (!user) {
    return null // RequireAuth guarantees a user; this satisfies the type checker
  }

  async function handleLogout() {
    await logout()
    router.replace("/")
  }

  function handleViewChange(view: DriveView) {
    setActiveView(view)
    setDialogError(null)
    setCurrentFolderId(null)
  }

  async function handleUpload(
    file: File,
    options: {
      parentFolderId?: string | null
      pendingUpload?: PendingResumableUpload
    } = {}
  ) {
    if (!token) {
      setError("Your session expired. Log in again to upload files.")
      return
    }

    setError(null)
    setActiveView("my-drive")
    setUploadingName(file.name)
    setUploadProgress(0)
    setUploadParts(null)
    setResumedFromPart(null)

    try {
      const parentFolderId =
        options.pendingUpload?.upload.parent_folder_id ??
        options.parentFolderId ??
        currentFolderId
      const contentType = file.type || "application/octet-stream"
      const storageKey =
        options.pendingUpload?.key ?? resumableUploadKey(file, parentFolderId)
      const stored =
        options.pendingUpload?.upload ?? readStoredUploads()[storageKey]
      let fileId: string | undefined = stored?.file_id
      let partSizeBytes = 0
      let confirmedParts = new Map<number, number>()

      if (fileId) {
        try {
          const status = await api.getUploadStatus(token, fileId)
          if (
            status.state === "pending" &&
            status.filename === file.name &&
            status.size_bytes === file.size &&
            status.parent_folder_id === parentFolderId &&
            new Date(status.expires_at).getTime() > Date.now()
          ) {
            partSizeBytes = status.part_size_bytes
            confirmedParts = new Map(
              status.parts.map((part) => [part.part_number, part.size_bytes])
            )
          } else {
            forgetUpload(storageKey)
            refreshPendingUploads()
            fileId = undefined
          }
        } catch {
          forgetUpload(storageKey)
          refreshPendingUploads()
          fileId = undefined
        }
      }

      if (!fileId) {
        const created = await api.createResumableUpload(token, {
          filename: file.name,
          parent_folder_id: parentFolderId,
          content_type: contentType,
          size_bytes: file.size,
          checksum_sha256: null,
        })
        fileId = created.file_id
        partSizeBytes = created.part_size_bytes
        rememberUpload(storageKey, {
          file_id: created.file_id,
          filename: file.name,
          size_bytes: file.size,
          last_modified: file.lastModified,
          content_type: contentType,
          parent_folder_id: parentFolderId,
          expires_at: created.expires_at,
        })
        refreshPendingUploads()
      }

      const confirmedList = Array.from(confirmedParts.entries()).map(
        ([part_number, size_bytes]) => ({ part_number, size_bytes })
      )
      const progress = resumedProgress(confirmedList, file.size, partSizeBytes)
      let uploadedBytes = progress.bytesDone
      const totalParts = progress.partsTotal
      let partsDone = progress.partsDone
      setUploadProgress(progress.percent)
      setUploadParts({ done: partsDone, total: totalParts })
      if (partsDone > 0) {
        // The next part to actually send is the first one not yet confirmed.
        let nextPart = 1
        while (confirmedParts.has(nextPart)) {
          nextPart += 1
        }
        setResumedFromPart(nextPart)
      }

      for (let partNumber = 1; partNumber <= totalParts; partNumber += 1) {
        if (confirmedParts.has(partNumber)) {
          continue
        }
        const start = (partNumber - 1) * partSizeBytes
        const end = Math.min(start + partSizeBytes, file.size)
        const blob = file.slice(start, end)
        const signed = await api.presignUploadPart(token, fileId, partNumber)
        const etag = await uploadFilePart(
          signed.upload_url,
          blob,
          partNumber,
          (loaded) => {
            const current = uploadedBytes + loaded
            setUploadProgress(Math.round((current / file.size) * 100))
          }
        )
        await api.recordUploadPart(token, fileId, partNumber, {
          size_bytes: blob.size,
          etag,
        })
        uploadedBytes += blob.size
        partsDone += 1
        setUploadProgress(Math.round((uploadedBytes / file.size) * 100))
        setUploadParts({ done: partsDone, total: totalParts })
      }

      await api.finalizeResumableUpload(token, fileId)
      forgetUpload(storageKey)
      await refreshUser()
      const response = await api.browseDrive(token, currentFolderId)
      setFolders(response.folders)
      setFiles(response.files)
      setBreadcrumbs(response.breadcrumbs)
      await syncPendingUploads()
    } catch (caught) {
      setError(getUploadErrorMessage(caught))
      refreshPendingUploads()
    } finally {
      setUploadingName(null)
      setUploadProgress(0)
      setUploadParts(null)
      setResumedFromPart(null)
      if (inputRef.current) {
        inputRef.current.value = ""
      }
      if (resumeInputRef.current) {
        resumeInputRef.current.value = ""
      }
      setResumeTarget(null)
    }
  }

  function handleResumeUpload(pending: PendingResumableUpload) {
    setError(null)
    setResumeTarget(pending)
    window.setTimeout(() => resumeInputRef.current?.click(), 0)
  }

  function handleResumeFileSelected(file: File) {
    if (!resumeTarget) {
      return
    }
    if (!fileMatchesStoredUpload(file, resumeTarget)) {
      setError(
        `Select the same file again to resume ${resumeTarget.upload.filename}.`
      )
      setResumeTarget(null)
      if (resumeInputRef.current) {
        resumeInputRef.current.value = ""
      }
      return
    }
    void handleUpload(file, { pendingUpload: resumeTarget })
  }

  function handleDismissPendingUpload(pending: PendingResumableUpload) {
    forgetUpload(pending.key)
    void syncPendingUploads()
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

  async function handleDeleteFolder(folderId: string) {
    if (!token) {
      setError("Your session expired. Log in again to delete folders.")
      return
    }

    setError(null)
    setDeleteId(folderId)
    try {
      await api.deleteFolder(token, folderId)
      await refreshUser()
      if (currentFolderId === folderId) {
        setCurrentFolderId(null)
      }
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

  async function handleRestoreFolder(folderId: string) {
    if (!token) {
      setError("Your session expired. Log in again to restore folders.")
      return
    }

    setError(null)
    setRestoreId(folderId)
    try {
      await api.restoreFolder(token, folderId)
      await refreshUser()
      await loadActiveView()
    } catch (caught) {
      setError(getApiErrorMessage(caught))
    } finally {
      setRestoreId(null)
    }
  }

  async function handleCreateFolder(name: string) {
    if (!token) {
      setDialogError("Your session expired. Log in again to create folders.")
      return
    }

    setDialogSubmitting(true)
    setDialogError(null)
    try {
      await api.createFolder(token, {
        name: name.trim(),
        parent_folder_id: currentFolderId,
      })
      setCreateFolderOpen(false)
      await loadActiveView()
    } catch (caught) {
      setDialogError(getApiErrorMessage(caught))
    } finally {
      setDialogSubmitting(false)
    }
  }

  async function loadMoveFolders() {
    if (!token) {
      setDialogError("Your session expired. Log in again to move items.")
      return
    }
    try {
      const response = await api.listFolders(token)
      setAllFolders(response.folders)
    } catch (caught) {
      setDialogError(getApiErrorMessage(caught))
    }
  }

  async function handleRename(name: string) {
    if (!token || !renameTarget) {
      setDialogError("Your session expired. Log in again to rename items.")
      return
    }

    setDialogSubmitting(true)
    setDialogError(null)
    try {
      if (renameTarget.kind === "file") {
        await api.updateFile(token, renameTarget.item.id, {
          filename: name.trim(),
        })
      } else {
        await api.updateFolder(token, renameTarget.item.id, {
          name: name.trim(),
        })
      }
      setRenameTarget(null)
      await loadActiveView()
    } catch (caught) {
      setDialogError(getApiErrorMessage(caught))
    } finally {
      setDialogSubmitting(false)
    }
  }

  async function handleMove(parentFolderId: string | null) {
    if (!token || !moveTarget) {
      setDialogError("Your session expired. Log in again to move items.")
      return
    }

    setDialogSubmitting(true)
    setDialogError(null)
    try {
      if (moveTarget.kind === "file") {
        await api.updateFile(token, moveTarget.item.id, {
          parent_folder_id: parentFolderId,
        })
      } else {
        await api.updateFolder(token, moveTarget.item.id, {
          parent_folder_id: parentFolderId,
        })
      }
      setMoveTarget(null)
      await loadActiveView()
    } catch (caught) {
      setDialogError(getApiErrorMessage(caught))
    } finally {
      setDialogSubmitting(false)
    }
  }

  return (
    <SidebarProvider>
      <CommandMenuProvider
        onViewChange={handleViewChange}
        onDownload={(fileId) => handleDownload(fileId)}
      >
        <AppSidebar activeView={activeView} onViewChange={handleViewChange} />
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
                  <CardDescription>Folders here</CardDescription>
                  <CardTitle className="font-heading text-2xl">
                    {folders.length}
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
            <input
              ref={resumeInputRef}
              type="file"
              className="sr-only"
              onChange={(event) => {
                const file = event.target.files?.[0]
                if (file) {
                  handleResumeFileSelected(file)
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
                <CardContent className="flex flex-col gap-2">
                  <Progress
                    value={uploadProgress}
                    aria-label="Upload progress"
                  />
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
            )}

            {pendingUploads.length > 0 && !uploadingName && (
              <Card size="sm">
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
                      onClick={() => void handleClearExpiredUploads()}
                    >
                      {clearingExpired ? "Clearing…" : "Clear expired"}
                    </Button>
                  </CardAction>
                </CardHeader>
                <CardContent className="flex flex-col gap-3">
                  <p className="text-sm text-muted-foreground">
                    Select the same local file again to continue from the parts
                    already saved.
                  </p>
                  <div className="flex flex-col gap-2">
                    {pendingUploads.map((pending) => {
                      const server = pending.server
                      const partsTotal =
                        server && server.part_size_bytes > 0
                          ? Math.ceil(
                              server.size_bytes / server.part_size_bytes
                            )
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
                            {formatDate(pending.upload.expires_at)}
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
                            variant="outline"
                            onClick={() => handleDismissPendingUpload(pending)}
                          >
                            Dismiss
                          </Button>
                          <Button
                            type="button"
                            onClick={() => handleResumeUpload(pending)}
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
            )}

            {activeView === "my-drive" && (
              <div className="flex flex-wrap items-center gap-1 text-sm text-muted-foreground">
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => setCurrentFolderId(null)}
                >
                  My Drive
                </Button>
                {breadcrumbs.map((entry) => (
                  <div key={entry.id} className="flex items-center gap-1">
                    <ChevronRight aria-hidden="true" />
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={() => setCurrentFolderId(entry.id)}
                    >
                      {entry.name}
                    </Button>
                  </div>
                ))}
              </div>
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
                    <>
                      <Button
                        variant="outline"
                        onClick={() => {
                          setDialogError(null)
                          setCreateFolderOpen(true)
                        }}
                        disabled={Boolean(uploadingName)}
                      >
                        <FolderPlus data-icon="inline-start" />
                        New folder
                      </Button>
                      <Button
                        onClick={() => inputRef.current?.click()}
                        disabled={Boolean(uploadingName)}
                      >
                        <Upload data-icon="inline-start" />
                        Upload
                      </Button>
                    </>
                  )}
                </CardAction>
              </CardHeader>
              <CardContent>
                {loadingFiles ? (
                  <div className="flex items-center gap-2 text-sm text-muted-foreground">
                    <Spinner />
                    Loading files…
                  </div>
                ) : visibleFiles.length === 0 && visibleFolders.length === 0 ? (
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
                      <EmptyDescription>
                        {copy.emptyDescription}
                      </EmptyDescription>
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
                    {visibleFolders.map((folder) => (
                      <div
                        key={folder.id}
                        className="flex items-center gap-3 rounded-lg border border-border p-3"
                      >
                        <button
                          type="button"
                          className="flex size-9 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground"
                          onClick={() => {
                            if (activeView === "my-drive") {
                              setCurrentFolderId(folder.id)
                            }
                          }}
                          aria-label={`Open ${folder.name}`}
                        >
                          <Folder aria-hidden="true" />
                        </button>
                        <div className="min-w-0 flex-1">
                          <button
                            type="button"
                            className="truncate font-medium"
                            onClick={() => {
                              if (activeView === "my-drive") {
                                setCurrentFolderId(folder.id)
                              }
                            }}
                          >
                            {folder.name}
                          </button>
                          <div className="flex flex-wrap gap-x-3 gap-y-1 text-xs text-muted-foreground">
                            {activeView === "trash" ? (
                              <span>
                                Deleted {formatDate(folder.deleted_at)}
                              </span>
                            ) : (
                              <span>
                                Created {formatDate(folder.created_at)}
                              </span>
                            )}
                          </div>
                        </div>
                        {activeView === "my-drive" ? (
                          <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                              <Button
                                variant="outline"
                                size="icon"
                                aria-label="Folder actions"
                              >
                                <MoreHorizontal />
                              </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end" className="w-40">
                              <DropdownMenuGroup>
                                <DropdownMenuItem
                                  onSelect={() => setCurrentFolderId(folder.id)}
                                >
                                  <FolderOpen />
                                  Open
                                </DropdownMenuItem>
                                <DropdownMenuItem
                                  onSelect={() => {
                                    setDialogError(null)
                                    setRenameTarget({
                                      kind: "folder",
                                      item: folder,
                                    })
                                  }}
                                >
                                  <Pencil />
                                  Rename
                                </DropdownMenuItem>
                                <DropdownMenuItem
                                  onSelect={() => {
                                    setDialogError(null)
                                    setMoveTarget({
                                      kind: "folder",
                                      item: folder,
                                    })
                                    void loadMoveFolders()
                                  }}
                                >
                                  <MoveRight />
                                  Move
                                </DropdownMenuItem>
                              </DropdownMenuGroup>
                              <DropdownMenuSeparator />
                              <DropdownMenuGroup>
                                <DropdownMenuItem
                                  variant="destructive"
                                  onSelect={() =>
                                    void handleDeleteFolder(folder.id)
                                  }
                                  disabled={deleteId === folder.id}
                                >
                                  <Trash2 />
                                  Delete
                                </DropdownMenuItem>
                              </DropdownMenuGroup>
                            </DropdownMenuContent>
                          </DropdownMenu>
                        ) : (
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => void handleRestoreFolder(folder.id)}
                            disabled={restoreId === folder.id}
                          >
                            {restoreId === folder.id ? (
                              <Spinner data-icon="inline-start" />
                            ) : (
                              <RotateCcw data-icon="inline-start" />
                            )}
                            Restore
                          </Button>
                        )}
                      </div>
                    ))}
                    {visibleFiles.map((file) => (
                      <div
                        key={file.id}
                        className="flex items-center gap-3 rounded-lg border border-border p-3"
                      >
                        <div className="flex size-9 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground">
                          <FileIcon aria-hidden="true" />
                        </div>
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center gap-2">
                            <span className="truncate font-medium">
                              {file.filename}
                            </span>
                            <Badge variant="secondary">{file.state}</Badge>
                          </div>
                          <div className="flex flex-wrap gap-x-3 gap-y-1 text-xs text-muted-foreground">
                            <span>{formatBytes(file.size_bytes)}</span>
                            {isSharedFile(file) ? (
                              <span>
                                Owner {file.owner.display_name} (
                                {file.owner.email})
                              </span>
                            ) : activeView === "trash" ? (
                              <span>Deleted {formatDate(file.deleted_at)}</span>
                            ) : (
                              <span>Created {formatDate(file.created_at)}</span>
                            )}
                            <span>
                              Completed {formatDate(file.completed_at)}
                            </span>
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
                                onClick={() => {
                                  setDialogError(null)
                                  setRenameTarget({ kind: "file", item: file })
                                }}
                              >
                                <Pencil data-icon="inline-start" />
                                Rename
                              </Button>
                              <Button
                                variant="outline"
                                size="sm"
                                onClick={() => {
                                  setDialogError(null)
                                  setMoveTarget({ kind: "file", item: file })
                                  void loadMoveFolders()
                                }}
                              >
                                <MoveRight data-icon="inline-start" />
                                Move
                              </Button>
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
          <CreateFolderDialog
            open={createFolderOpen}
            submitting={dialogSubmitting}
            error={dialogError}
            onOpenChange={(open) => {
              setCreateFolderOpen(open)
              setDialogError(null)
            }}
            onSubmit={(name) => void handleCreateFolder(name)}
          />
          <RenameDialog
            target={renameTarget}
            submitting={dialogSubmitting}
            error={dialogError}
            onOpenChange={(open) => {
              if (!open) {
                setRenameTarget(null)
                setDialogError(null)
              }
            }}
            onSubmit={(name) => void handleRename(name)}
          />
          <MoveDialog
            target={moveTarget}
            folders={allFolders}
            submitting={dialogSubmitting}
            error={dialogError}
            onOpenChange={(open) => {
              if (!open) {
                setMoveTarget(null)
                setDialogError(null)
              }
            }}
            onSubmit={(parentFolderId) => void handleMove(parentFolderId)}
          />
        </SidebarInset>
      </CommandMenuProvider>
    </SidebarProvider>
  )
}
