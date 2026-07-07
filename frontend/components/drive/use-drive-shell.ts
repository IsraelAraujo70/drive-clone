"use client"

import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { useRouter } from "next/navigation"

import type { DriveItem, DriveView } from "@/components/drive/types"
import {
  api,
  uploadFilePart,
  type FileRecord,
  type FolderPathEntry,
  type FolderRecord,
  type SharedFileRecord,
} from "@/lib/api"
import { useAuth } from "@/lib/auth"
import { getVisibleDriveItems } from "@/lib/driveView"
import {
  clearExpiredUploads,
  dismissUpload,
  fileMatchesStoredUpload,
  forgetUpload,
  hasClearableExpiredUploads,
  mergePendingUploads,
  pendingStoredUploads,
  readStoredUploads,
  rememberUpload,
  resumableUploadKey,
  resumedProgress,
  type PendingResumableUpload,
  type ServerPendingUpload,
} from "@/lib/resumableUploads"
import { getApiErrorMessage } from "@/lib/shareErrors"
import { getUploadErrorMessage } from "@/lib/uploadErrors"

export function useDriveShell() {
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
  const [canClearExpiredUploads, setCanClearExpiredUploads] = useState(false)
  const [resumeTarget, setResumeTarget] =
    useState<PendingResumableUpload | null>(null)
  const [downloadId, setDownloadId] = useState<string | null>(null)
  const [deleteId, setDeleteId] = useState<string | null>(null)
  const [restoreId, setRestoreId] = useState<string | null>(null)
  const [purgeId, setPurgeId] = useState<string | null>(null)
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
  const { visibleFiles, visibleFolders } = useMemo(
    () =>
      getVisibleDriveItems({
        activeView,
        files,
        folders,
        trashFiles,
        trashFolders,
        sharedFiles,
      }),
    [activeView, files, folders, sharedFiles, trashFiles, trashFolders]
  )

  const refreshPendingUploads = useCallback(() => {
    setPendingUploads(pendingStoredUploads())
    setCanClearExpiredUploads(hasClearableExpiredUploads())
  }, [])

  const syncPendingUploads = useCallback(async () => {
    if (!token) {
      setPendingUploads([])
      return
    }
    try {
      const response = await api.listPendingUploads(token)
      const serverUploads: ServerPendingUpload[] = response.uploads
      const activeFileIds = new Set(
        serverUploads.map((upload) => upload.file_id)
      )
      setCanClearExpiredUploads(hasClearableExpiredUploads(activeFileIds))
      setPendingUploads(mergePendingUploads(serverUploads))
    } catch {
      setCanClearExpiredUploads(hasClearableExpiredUploads())
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
    dismissUpload(pending)
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
      window.open(response.download_url, "_self")
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

  async function handlePurge(file: FileRecord) {
    if (!token) {
      setError("Your session expired. Log in again to delete files.")
      return
    }
    if (
      !window.confirm(
        `Permanently delete ${file.filename}? This cannot be undone.`
      )
    ) {
      return
    }

    setError(null)
    setPurgeId(file.id)
    try {
      await api.purgeFile(token, file.id)
      await refreshUser()
      await loadActiveView()
    } catch (caught) {
      setError(getApiErrorMessage(caught))
    } finally {
      setPurgeId(null)
    }
  }

  async function handlePurgeFolder(folder: FolderRecord) {
    if (!token) {
      setError("Your session expired. Log in again to delete folders.")
      return
    }
    if (
      !window.confirm(
        `Permanently delete ${folder.name} and everything inside it? This cannot be undone.`
      )
    ) {
      return
    }

    setError(null)
    setPurgeId(folder.id)
    try {
      await api.purgeFolder(token, folder.id)
      await refreshUser()
      await loadActiveView()
    } catch (caught) {
      setError(getApiErrorMessage(caught))
    } finally {
      setPurgeId(null)
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

  function openCreateFolderDialog() {
    setDialogError(null)
    setCreateFolderOpen(true)
  }

  function openRenameDialog(target: DriveItem) {
    setDialogError(null)
    setRenameTarget(target)
  }

  function openMoveDialog(target: DriveItem) {
    setDialogError(null)
    setMoveTarget(target)
    void loadMoveFolders()
  }

  return {
    activeView,
    allFolders,
    breadcrumbs,
    canClearExpiredUploads,
    clearingExpired,
    completedFilesCount: completedFiles.length,
    createFolderOpen,
    deleteId,
    dialogError,
    dialogSubmitting,
    downloadId,
    error,
    foldersCount: folders.length,
    inputRef,
    loadingFiles,
    moveTarget,
    pendingUploads,
    purgeId,
    renameTarget,
    restoreId,
    resumeInputRef,
    shareFile,
    token,
    uploadParts,
    uploadProgress,
    uploadingName,
    user,
    visibleFiles,
    visibleFolders,
    resumedFromPart,
    handleClearExpiredUploads,
    handleCreateFolder,
    handleDelete,
    handleDeleteFolder,
    handleDismissPendingUpload,
    handleDownload,
    handleLogout,
    handleMove,
    handlePurge,
    handlePurgeFolder,
    handleRename,
    handleRestore,
    handleRestoreFolder,
    handleResumeFileSelected,
    handleResumeUpload,
    handleUpload,
    handleViewChange,
    openCreateFolderDialog,
    openMoveDialog,
    openRenameDialog,
    setCreateFolderOpen,
    setCurrentFolderId,
    setDialogError,
    setMoveTarget,
    setRenameTarget,
    setShareFile,
    loadActiveView,
  }
}
