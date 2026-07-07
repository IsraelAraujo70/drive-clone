import { Badge } from "@/components/ui/badge"
import type { FileRecord } from "@/lib/api"

export function FileStateBadge({ state }: { state: FileRecord["state"] }) {
  return <Badge variant="secondary">{state}</Badge>
}
