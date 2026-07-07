import { ChevronRight } from "lucide-react"

import { Button } from "@/components/ui/button"
import type { FolderPathEntry } from "@/lib/api"

export function DriveBreadcrumbs({
  breadcrumbs,
  onRootClick,
  onFolderClick,
}: {
  breadcrumbs: FolderPathEntry[]
  onRootClick: () => void
  onFolderClick: (folderId: string) => void
}) {
  return (
    <div className="flex flex-wrap items-center gap-1 text-sm text-muted-foreground">
      <Button type="button" variant="ghost" size="sm" onClick={onRootClick}>
        My Drive
      </Button>
      {breadcrumbs.map((entry) => (
        <div key={entry.id} className="flex items-center gap-1">
          <ChevronRight aria-hidden="true" />
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => onFolderClick(entry.id)}
          >
            {entry.name}
          </Button>
        </div>
      ))}
    </div>
  )
}
