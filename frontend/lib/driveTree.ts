import type { FolderRecord } from "@/lib/api"

export function isDescendantFolder(
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
