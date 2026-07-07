"use client"

import { useCallback, useEffect, useState, type FormEvent } from "react"

import { ShareGranteeList } from "@/components/drive/molecules/share-grantee-list"
import { SharePublicLinkList } from "@/components/drive/molecules/share-public-link-list"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import {
  Field,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { Separator } from "@/components/ui/separator"
import { Spinner } from "@/components/ui/spinner"
import {
  api,
  type FileRecord,
  type Share,
  type ShareLinkRecord,
} from "@/lib/api"
import { getApiErrorMessage, getShareErrorMessage } from "@/lib/shareErrors"

export function ShareDialog({
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
  const [links, setLinks] = useState<ShareLinkRecord[]>([])
  const [email, setEmail] = useState("")
  const [loading, setLoading] = useState(false)
  const [submitting, setSubmitting] = useState(false)
  const [revokeId, setRevokeId] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [creatingLink, setCreatingLink] = useState(false)
  const [copiedLinkId, setCopiedLinkId] = useState<string | null>(null)
  const [revokeLinkId, setRevokeLinkId] = useState<string | null>(null)
  const [linkError, setLinkError] = useState<string | null>(null)

  const loadShares = useCallback(async () => {
    if (!token || !file) {
      setShares([])
      setLinks([])
      return
    }

    setLoading(true)
    setError(null)
    setLinkError(null)
    try {
      const [shareResponse, linkResponse] = await Promise.all([
        api.listShares(token, file.id),
        api.listShareLinks(token, file.id),
      ])
      setShares(shareResponse.shares)
      setLinks(linkResponse.links.filter((link) => link.revoked_at === null))
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
      setLinks([])
      setLinkError(null)
      setCopiedLinkId(null)
    }
    onOpenChange(nextOpen)
  }

  async function handleCreateLink() {
    if (!token || !file) {
      setLinkError("Your session expired. Log in again to create a link.")
      return
    }

    setCreatingLink(true)
    setLinkError(null)
    try {
      const created = await api.createShareLink(token, file.id)
      setLinks((current) => [
        {
          id: created.id,
          created_at: new Date().toISOString(),
          expires_at: created.expires_at,
          revoked_at: null,
        },
        ...current,
      ])
      try {
        await navigator.clipboard.writeText(created.url)
        setCopiedLinkId(created.id)
      } catch {
        // Clipboard can be blocked; the link still exists and is listed.
        setCopiedLinkId(null)
      }
    } catch (caught) {
      setLinkError(getApiErrorMessage(caught))
    } finally {
      setCreatingLink(false)
    }
  }

  async function handleRevokeLink(linkId: string) {
    if (!token || !file) {
      setLinkError("Your session expired. Log in again to revoke a link.")
      return
    }

    setRevokeLinkId(linkId)
    setLinkError(null)
    try {
      await api.revokeShareLink(token, file.id, linkId)
      setLinks((current) => current.filter((link) => link.id !== linkId))
    } catch (caught) {
      setLinkError(getApiErrorMessage(caught))
    } finally {
      setRevokeLinkId(null)
    }
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
                  data-cy="share-email-input"
                  type="email"
                  value={email}
                  onChange={(event) => setEmail(event.target.value)}
                  placeholder="friend@example.com"
                  aria-invalid={Boolean(error)}
                  disabled={submitting}
                  required
                />
                <Button
                  type="submit"
                  data-cy="share-submit"
                  disabled={submitting || !email.trim()}
                >
                  {submitting && <Spinner data-icon="inline-start" />}
                  Share
                </Button>
              </div>
              {error && <FieldError>{error}</FieldError>}
            </Field>
          </FieldGroup>
        </form>

        <ShareGranteeList
          loading={loading}
          revokeId={revokeId}
          shares={shares}
          onRevoke={(granteeId) => void handleRevoke(granteeId)}
        />

        <Separator />

        <SharePublicLinkList
          copiedLinkId={copiedLinkId}
          creatingLink={creatingLink}
          fileSelected={Boolean(file)}
          linkError={linkError}
          links={links}
          revokeLinkId={revokeLinkId}
          onCreateLink={() => void handleCreateLink()}
          onRevokeLink={(linkId) => void handleRevokeLink(linkId)}
        />

        <DialogFooter showCloseButton />
      </DialogContent>
    </Dialog>
  )
}
