import { FileIcon, Folder } from "lucide-react"

export function DriveItemIcon({ kind }: { kind: "file" | "folder" }) {
  const Icon = kind === "folder" ? Folder : FileIcon

  return (
    <div className="flex size-9 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground">
      <Icon aria-hidden="true" />
    </div>
  )
}
