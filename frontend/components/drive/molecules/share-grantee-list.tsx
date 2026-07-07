import { Button } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import type { Share } from "@/lib/api"

export function ShareGranteeList({
  loading,
  revokeId,
  shares,
  onRevoke,
}: {
  loading: boolean
  revokeId: string | null
  shares: Share[]
  onRevoke: (granteeId: string) => void
}) {
  return (
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
              onClick={() => onRevoke(share.grantee.id)}
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
  )
}
