type LandingFeatureRowProps = {
  path: string
  title: string
  copy: string
}

export function LandingFeatureRow({
  path,
  title,
  copy,
}: LandingFeatureRowProps) {
  return (
    <div className="grid items-baseline gap-x-8 gap-y-1 border-b border-border/70 py-5 md:grid-cols-[140px_240px_1fr]">
      <span className="font-mono text-sm text-primary">{path}</span>
      <h2 className="font-heading text-lg font-semibold">{title}</h2>
      <p className="text-muted-foreground">{copy}</p>
    </div>
  )
}
