export const TRASH_RETENTION_DAYS = 30

export const trashPolicyCopy = {
  description: `Deleted files can be restored for ${TRASH_RETENTION_DAYS} days before they are permanently deleted.`,
  emptyDescription: `Deleted files will appear here for ${TRASH_RETENTION_DAYS} days unless you restore them first.`,
}
