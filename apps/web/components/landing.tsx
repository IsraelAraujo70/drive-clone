"use client"

import Link from "next/link"

import { Brand } from "@/components/brand"
import { Button } from "@/components/ui/button"
import { Progress } from "@/components/ui/progress"
import { useAuth } from "@/lib/auth"

const listing = [
  {
    path: "/uploads",
    title: "Signed direct uploads",
    copy: "The API checks quota, signs a short-lived upload URL, then verifies the stored object before it appears.",
  },
  {
    path: "/folders",
    title: "Organize without fear",
    copy: "Rename, move, trash and restore. Deleted files wait in trash until you decide.",
  },
  {
    path: "/shared",
    title: "Private by default",
    copy: "Grant access to a registered user's email, revoke that access, and keep every other file private.",
  },
  {
    path: "/search",
    title: "Found by name, instantly",
    copy: "Search everything you own. Results never include files you cannot access.",
  },
]

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

function UploadCard() {
  return (
    <figure
      role="img"
      aria-label="Current Drive Clone upload flow: create upload metadata, upload directly to object storage, complete the file, then show it in the drive."
      className="border-border bg-card relative mt-6 rounded-xl border p-6 shadow-sm"
    >
      <span className="bg-manila text-manila-foreground absolute -top-6 left-6 rounded-t-md px-3 py-1 font-mono text-[11px] font-medium tracking-[0.14em] uppercase">
        drive / uploads
      </span>

      <div className="flex items-baseline justify-between gap-4 font-mono">
        <span className="text-sm font-medium">contract.pdf</span>
        <span className="text-muted-foreground text-xs">single object</span>
      </div>

      <div className="mt-5 grid gap-3">
        {uploadFlow.map((step, index) => (
          <div
            key={step.path}
            className="border-border/80 grid min-h-16 grid-cols-[44px_1fr] items-start gap-3 border-b pb-3 last:border-b-0 last:pb-0"
          >
            <span className="bg-secondary text-primary flex size-8 items-center justify-center rounded-full font-mono text-xs font-semibold">
              {index + 1}
            </span>
            <div className="min-w-0">
              <p className="font-mono text-xs font-medium">
                <span className="text-success">{step.method}</span>{" "}
                <span className="break-all">{step.path}</span>
              </p>
              <p className="text-muted-foreground mt-1 text-sm leading-snug">
                {step.detail}
              </p>
            </div>
          </div>
        ))}
      </div>

      <figcaption className="mt-5 flex items-center gap-3 font-mono text-xs">
        <span className="bg-success/10 text-success rounded-full px-2.5 py-0.5 font-medium tracking-wider uppercase">
          complete only
        </span>
        <span className="text-muted-foreground">
          Pending objects stay hidden until verified.
        </span>
      </figcaption>
    </figure>
  )
}

export function Landing() {
  const { user } = useAuth()
  const primaryHref = user ? "/drive" : "/signup"
  const primaryLabel = user ? "Open your drive" : "Create your drive"

  return (
    <div className="flex min-h-svh flex-col">
      <header className="border-border bg-card border-b">
        <div className="mx-auto flex w-full max-w-6xl items-center justify-between gap-4 px-6 py-4">
          <Brand />
          <nav aria-label="Account" className="flex items-center gap-2">
            {user ? (
              <Button asChild>
                <Link href="/drive">Open your drive</Link>
              </Button>
            ) : (
              <>
                <Button variant="ghost" asChild>
                  <Link href="/login">Log in</Link>
                </Button>
                <Button asChild>
                  <Link href="/signup">Sign up</Link>
                </Button>
              </>
            )}
          </nav>
        </div>
      </header>

      <main className="flex-1">
        <section className="mx-auto grid w-full max-w-6xl items-center gap-14 px-6 pt-20 pb-16 lg:grid-cols-[1.1fr_1fr]">
          <div className="flex flex-col gap-6">
            <p className="text-muted-foreground flex items-center gap-2 font-mono text-xs tracking-[0.18em] uppercase">
              <span aria-hidden="true" className="bg-manila inline-block size-2" />
              Personal cloud storage · 15 GB free
            </p>
            <h1 className="font-heading text-[clamp(2.75rem,6vw,4.5rem)] leading-[0.98] font-bold tracking-tight">
              Uploads that finish.
              <br />
              Files that stay yours.
            </h1>
            <p className="text-muted-foreground max-w-[34rem] text-lg leading-relaxed">
              Drive Clone signs direct uploads, verifies stored objects before
              listing them, and keeps files private unless you grant access to a
              registered account.
            </p>
            <div className="flex flex-wrap items-center gap-3">
              <Button size="lg" asChild>
                <Link href={primaryHref}>{primaryLabel}</Link>
              </Button>
              {!user && (
                <Button size="lg" variant="outline" asChild>
                  <Link href="/login">Log in</Link>
                </Button>
              )}
            </div>
          </div>
          <UploadCard />
        </section>

        <section
          aria-label="What your drive can do"
          className="mx-auto w-full max-w-6xl px-6 pb-24"
        >
          <div className="border-border text-muted-foreground flex items-center justify-between border-b pb-3 font-mono text-xs tracking-wider uppercase">
            <span>~/drive</span>
            <span>{listing.length} entries</span>
          </div>
          {listing.map((entry) => (
            <div
              key={entry.path}
              className="border-border/70 grid items-baseline gap-x-8 gap-y-1 border-b py-5 md:grid-cols-[140px_240px_1fr]"
            >
              <span className="text-primary font-mono text-sm">{entry.path}</span>
              <h2 className="font-heading text-lg font-semibold">{entry.title}</h2>
              <p className="text-muted-foreground">{entry.copy}</p>
            </div>
          ))}
        </section>

        <section className="border-border bg-card border-t">
          <div className="mx-auto flex w-full max-w-6xl flex-col gap-8 px-6 py-16 lg:flex-row lg:items-center lg:justify-between">
            <div className="flex max-w-md flex-col gap-3">
              <h2 className="font-heading text-3xl font-bold tracking-tight">
                15 GB free, counted in bytes.
              </h2>
              <p className="text-muted-foreground">
                Every account starts with the full quota. No credit card, no
                trial clock.
              </p>
            </div>
            <div className="flex w-full max-w-sm flex-col gap-2">
              <Progress value={100} aria-label="Free storage included" />
              <span className="text-muted-foreground font-mono text-xs">
                storage_quota_bytes = 16,106,127,360
              </span>
            </div>
            <Button size="lg" asChild>
              <Link href={primaryHref}>{primaryLabel}</Link>
            </Button>
          </div>
        </section>
      </main>

      <footer className="border-border text-muted-foreground border-t py-6 text-center font-mono text-xs">
        A portfolio cloud-storage project · Rust API · Next.js web · PostgreSQL
        metadata · S3-compatible objects
      </footer>
    </div>
  )
}
