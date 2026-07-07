const uploadFlow = [
  {
    method: "POST",
    path: "/files/uploads",
    detail: "Validate metadata, parent folder, size, and quota.",
  },
  {
    method: "PUT",
    path: "upload_url",
    detail: "Send bytes directly to S3-compatible object storage.",
  },
  {
    method: "POST",
    path: "/files/{file_id}/complete",
    detail: "Verify object length and mark the file complete.",
  },
  {
    method: "GET",
    path: "/drive",
    detail: "Show only completed files the user can access.",
  },
]

export function UploadFlowCard() {
  return (
    <figure
      role="img"
      aria-label="Current Drive Clone upload flow: create upload metadata, upload directly to object storage, complete the file, then show it in the drive."
      className="relative mt-6 rounded-xl border border-border bg-card p-6 shadow-sm"
    >
      <span className="absolute -top-6 left-6 rounded-t-md bg-manila px-3 py-1 font-mono text-[11px] font-medium tracking-[0.14em] text-manila-foreground uppercase">
        drive / uploads
      </span>

      <div className="flex items-baseline justify-between gap-4 font-mono">
        <span className="text-sm font-medium">contract.pdf</span>
        <span className="text-xs text-muted-foreground">single object</span>
      </div>

      <div className="mt-5 grid gap-3">
        {uploadFlow.map((step, index) => (
          <div
            key={step.path}
            className="grid min-h-16 grid-cols-[44px_1fr] items-start gap-3 border-b border-border/80 pb-3 last:border-b-0 last:pb-0"
          >
            <span className="flex size-8 items-center justify-center rounded-full bg-secondary font-mono text-xs font-semibold text-primary">
              {index + 1}
            </span>
            <div className="min-w-0">
              <p className="font-mono text-xs font-medium">
                <span className="text-success">{step.method}</span>{" "}
                <span className="break-all">{step.path}</span>
              </p>
              <p className="mt-1 text-sm leading-snug text-muted-foreground">
                {step.detail}
              </p>
            </div>
          </div>
        ))}
      </div>

      <figcaption className="mt-5 flex items-center gap-3 font-mono text-xs">
        <span className="rounded-full bg-success/10 px-2.5 py-0.5 font-medium tracking-wider text-success uppercase">
          complete only
        </span>
        <span className="text-muted-foreground">
          Pending objects stay hidden until verified.
        </span>
      </figcaption>
    </figure>
  )
}
