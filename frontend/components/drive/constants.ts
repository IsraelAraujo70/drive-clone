import type { DriveView } from "@/components/drive/types"
import { trashPolicyCopy } from "@/lib/trashPolicy"

export const viewCopy: Record<
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
    description: trashPolicyCopy.description,
    cardDescription: "Deleted files",
    cardTitle: "Trash",
    emptyTitle: "Trash is empty",
    emptyDescription: trashPolicyCopy.emptyDescription,
  },
}
