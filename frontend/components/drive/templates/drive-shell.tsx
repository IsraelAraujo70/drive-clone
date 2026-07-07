"use client"

import { AlertCircle } from "lucide-react"

import { CommandMenuProvider } from "@/components/command/organisms/command-menu-provider"
import { viewCopy } from "@/components/drive/constants"
import { DriveBreadcrumbs } from "@/components/drive/molecules/drive-breadcrumbs"
import { DriveStatCard } from "@/components/drive/molecules/drive-stat-card"
import { PendingUploadCard } from "@/components/drive/molecules/pending-upload-card"
import { UploadProgressCard } from "@/components/drive/molecules/upload-progress-card"
import { DriveBrowserCard } from "@/components/drive/organisms/drive-browser-card"
import { DriveDialogs } from "@/components/drive/organisms/drive-dialogs"
import { DriveHeader } from "@/components/drive/organisms/drive-header"
import { DriveHiddenFileInputs } from "@/components/drive/organisms/drive-hidden-file-inputs"
import { DriveSidebar } from "@/components/drive/organisms/drive-sidebar"
import { useDriveShell } from "@/components/drive/use-drive-shell"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar"
import { formatBytes } from "@/lib/format"

export function DriveShell() {
  const { inputRef, resumeInputRef, ...drive } = useDriveShell()

  if (!drive.user) {
    return null
  }

  const copy = viewCopy[drive.activeView]

  return (
    <SidebarProvider>
      <CommandMenuProvider
        onViewChange={drive.handleViewChange}
        onDownload={(fileId) => drive.handleDownload(fileId)}
      >
        <DriveSidebar
          activeView={drive.activeView}
          onViewChange={drive.handleViewChange}
        />
        <SidebarInset>
          <DriveHeader user={drive.user} onLogout={drive.handleLogout} />

          <main className="flex flex-col gap-5 p-6 md:p-7">
            <div>
              <h1 className="font-heading text-3xl font-bold tracking-tight">
                {copy.title}
              </h1>
              <p className="text-muted-foreground">{copy.description}</p>
            </div>

            <div className="grid gap-4 sm:grid-cols-3">
              <DriveStatCard
                label="My files"
                value={drive.completedFilesCount}
              />
              <DriveStatCard label="Folders here" value={drive.foldersCount} />
              <DriveStatCard
                label="Storage used"
                value={formatBytes(drive.user.storage_used_bytes)}
              />
            </div>

            <DriveHiddenFileInputs
              inputRef={inputRef}
              resumeInputRef={resumeInputRef}
              onResumeSelected={drive.handleResumeFileSelected}
              onUploadSelected={(file) => void drive.handleUpload(file)}
            />

            {drive.error && (
              <Alert variant="destructive">
                <AlertCircle />
                <AlertTitle>File action failed</AlertTitle>
                <AlertDescription>{drive.error}</AlertDescription>
              </Alert>
            )}

            {drive.uploadingName && (
              <UploadProgressCard
                uploadingName={drive.uploadingName}
                uploadProgress={drive.uploadProgress}
                uploadParts={drive.uploadParts}
                resumedFromPart={drive.resumedFromPart}
              />
            )}

            {drive.pendingUploads.length > 0 && !drive.uploadingName && (
              <PendingUploadCard
                pendingUploads={drive.pendingUploads}
                clearingExpired={drive.clearingExpired}
                onClearExpired={() => void drive.handleClearExpiredUploads()}
                onDismiss={drive.handleDismissPendingUpload}
                onResume={drive.handleResumeUpload}
              />
            )}

            {drive.activeView === "my-drive" && (
              <DriveBreadcrumbs
                breadcrumbs={drive.breadcrumbs}
                onRootClick={() => drive.setCurrentFolderId(null)}
                onFolderClick={drive.setCurrentFolderId}
              />
            )}

            <DriveBrowserCard
              activeView={drive.activeView}
              copy={copy}
              deleteId={drive.deleteId}
              downloadId={drive.downloadId}
              loadingFiles={drive.loadingFiles}
              purgeId={drive.purgeId}
              restoreId={drive.restoreId}
              uploadingName={drive.uploadingName}
              visibleFiles={drive.visibleFiles}
              visibleFolders={drive.visibleFolders}
              onCreateFolder={drive.openCreateFolderDialog}
              onDeleteFile={(fileId) => void drive.handleDelete(fileId)}
              onDeleteFolder={(folderId) =>
                void drive.handleDeleteFolder(folderId)
              }
              onDownload={(fileId) => void drive.handleDownload(fileId)}
              onMoveFile={(file) =>
                drive.openMoveDialog({ kind: "file", item: file })
              }
              onMoveFolder={(folder) =>
                drive.openMoveDialog({ kind: "folder", item: folder })
              }
              onOpenFolder={drive.setCurrentFolderId}
              onPurgeFile={(file) => void drive.handlePurge(file)}
              onPurgeFolder={(folder) => void drive.handlePurgeFolder(folder)}
              onReload={() => void drive.loadActiveView()}
              onRenameFile={(file) =>
                drive.openRenameDialog({ kind: "file", item: file })
              }
              onRenameFolder={(folder) =>
                drive.openRenameDialog({ kind: "folder", item: folder })
              }
              onRestoreFile={(fileId) => void drive.handleRestore(fileId)}
              onRestoreFolder={(folderId) =>
                void drive.handleRestoreFolder(folderId)
              }
              onShareFile={drive.setShareFile}
              onUpload={() => inputRef.current?.click()}
            />
          </main>

          <DriveDialogs
            allFolders={drive.allFolders}
            createFolderOpen={drive.createFolderOpen}
            dialogError={drive.dialogError}
            dialogSubmitting={drive.dialogSubmitting}
            moveTarget={drive.moveTarget}
            renameTarget={drive.renameTarget}
            shareFile={drive.shareFile}
            token={drive.token}
            onCreateFolder={(name) => void drive.handleCreateFolder(name)}
            onCreateFolderOpenChange={drive.setCreateFolderOpen}
            onDialogErrorChange={drive.setDialogError}
            onMove={(parentFolderId) => void drive.handleMove(parentFolderId)}
            onMoveTargetChange={drive.setMoveTarget}
            onRename={(name) => void drive.handleRename(name)}
            onRenameTargetChange={drive.setRenameTarget}
            onShareFileChange={drive.setShareFile}
          />
        </SidebarInset>
      </CommandMenuProvider>
    </SidebarProvider>
  )
}
