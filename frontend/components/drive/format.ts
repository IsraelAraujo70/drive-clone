const dateFormatter = new Intl.DateTimeFormat(undefined, {
  dateStyle: "medium",
  timeStyle: "short",
})

export function formatDriveDate(value: string | null): string {
  if (!value) {
    return "Pending"
  }
  return dateFormatter.format(new Date(value))
}
