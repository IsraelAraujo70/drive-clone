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
import { Spinner } from "@/components/ui/spinner"
import type { DriveItem } from "@/components/drive/types"

export function RenameDialog({
  target,
  submitting,
  error,
  onOpenChange,
  onSubmit,
}: {
  target: DriveItem | null
  submitting: boolean
  error: string | null
  onOpenChange: (open: boolean) => void
  onSubmit: (name: string) => void
}) {
  const defaultName =
    target?.kind === "file" ? target.item.filename : (target?.item.name ?? "")

  return (
    <Dialog open={Boolean(target)} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md" key={target?.item.id ?? "rename"}>
        <DialogHeader>
          <DialogTitle>Rename {target?.kind ?? "item"}</DialogTitle>
          <DialogDescription>
            Update the visible name in your drive.
          </DialogDescription>
        </DialogHeader>
        <form
          onSubmit={(event) => {
            event.preventDefault()
            const form = event.currentTarget
            const data = new FormData(form)
            onSubmit(String(data.get("name") ?? ""))
          }}
        >
          <FieldGroup>
            <Field data-invalid={Boolean(error)}>
              <FieldLabel htmlFor="rename-name">Name</FieldLabel>
              <Input
                id="rename-name"
                name="name"
                defaultValue={defaultName}
                aria-invalid={Boolean(error)}
                disabled={submitting}
                required
              />
              {error && <FieldError>{error}</FieldError>}
            </Field>
          </FieldGroup>
          <DialogFooter>
            <Button type="submit" disabled={submitting}>
              {submitting && <Spinner data-icon="inline-start" />}
              Rename
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
