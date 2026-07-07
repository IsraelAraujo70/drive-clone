import { Check, Link2 } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import type { ShareLinkRecord } from "@/lib/api"

export function SharePublicLinkList({
  copiedLinkId,
  creatingLink,
  fileSelected,
  linkError,
  links,
  revokeLinkId,
  onCreateLink,
  onRevokeLink,
}: {
  copiedLinkId: string | null
  creatingLink: boolean
  fileSelected: boolean
  linkError: string | null
  links: ShareLinkRecord[]
  revokeLinkId: string | null
  onCreateLink: () => void
  onRevokeLink: (linkId: string) => void
}) {
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between gap-2">
        <div>
          <div className="text-sm font-medium">Public link</div>
          <p className="text-xs text-muted-foreground">
            Anyone with the link can download this file until you revoke it.
          </p>
        </div>
        <Button
          type="button"
          data-cy="create-share-link"
          variant="outline"
          size="sm"
          onClick={onCreateLink}
          disabled={creatingLink || !fileSelected}
        >
          {creatingLink ? (
            <Spinner data-icon="inline-start" />
          ) : (
            <Link2 data-icon="inline-start" />
          )}
          Create link
        </Button>
      </div>

      {linkError && (
        <p className="text-sm text-destructive" role="alert">
          {linkError}
        </p>
      )}

      {links.length === 0 ? (
        <p className="text-sm text-muted-foreground">No public links yet.</p>
      ) : (
        links.map((link) => (
          <div
            key={link.id}
            data-cy="public-share-link"
            className="flex items-center gap-3 rounded-lg border border-border p-3"
          >
            <div className="min-w-0 flex-1">
              <div className="truncate text-sm font-medium">
                {copiedLinkId === link.id ? (
                  <span className="flex items-center gap-1 text-emerald-600">
                    <Check className="size-3.5" aria-hidden />
                    Copied to clipboard
                  </span>
                ) : (
                  "Public download link"
                )}
              </div>
              <div className="truncate text-xs text-muted-foreground">
                {link.expires_at
                  ? `Expires ${new Date(link.expires_at).toLocaleString()}`
                  : "No expiration"}
              </div>
            </div>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => onRevokeLink(link.id)}
              disabled={revokeLinkId === link.id}
            >
              {revokeLinkId === link.id && <Spinner data-icon="inline-start" />}
              Revoke
            </Button>
          </div>
        ))
      )}
    </div>
  )
}
