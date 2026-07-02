"use client"

import { useEffect, useState } from "react"
import Link from "next/link"

import { Brand } from "@/components/brand"
import { Button } from "@/components/ui/button"
import { Progress } from "@/components/ui/progress"
import { useAuth } from "@/lib/auth"
import {
  INTERRUPT_AT,
  TOTAL_CHUNKS,
  statusFor,
  uploadFrames,
  type UploadPhase,
} from "@/lib/uploadDemo"
import { cn } from "@/lib/utils"

const FRAME_MS = 130

const listing = [
  {
    path: "/uploads",
    title: "Resumable by design",
    copy: "Interrupted transfers pick up at the last confirmed chunk, never from zero.",
  },
  {
    path: "/folders",
    title: "Organize without fear",
    copy: "Rename, move, trash and restore. Deleted files wait in trash until you decide.",
  },
  {
    path: "/shared",
    title: "Private by default",
    copy: "Nothing is visible to anyone until you create a share link. Revoke it any time.",
  },
  {
    path: "/search",
    title: "Found by name, instantly",
    copy: "Search everything you own. Results never include files you cannot access.",
  },
]

function useUploadDemo() {
  // Static "complete" frame until the effect runs, and forever under reduced motion.
  const [index, setIndex] = useState(uploadFrames.length - 1)

  useEffect(() => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      return
    }
    // Starts at the last frame, so the first tick wraps to frame 0.
    const timer = setInterval(
      () => setIndex((current) => (current + 1) % uploadFrames.length),
      FRAME_MS,
    )
    return () => clearInterval(timer)
  }, [])

  return uploadFrames[index]
}

const phaseLabel: Record<UploadPhase, string> = {
  uploading: "receiving",
  interrupted: "interrupted",
  resumed: "receiving",
  complete: "complete",
}

const phasePill: Record<UploadPhase, string> = {
  uploading: "bg-secondary text-primary",
  resumed: "bg-secondary text-primary",
  interrupted: "bg-accent text-accent-foreground",
  complete: "bg-success/10 text-success",
}

function UploadCard() {
  const frame = useUploadDemo()
  const done = frame.phase === "complete"

  return (
    <figure
      role="img"
      aria-label="Demo of a resumable upload: the transfer loses its connection at chunk 12, resumes from the saved progress, and completes."
      className="border-border bg-card relative mt-6 rounded-xl border p-6 shadow-sm"
    >
      <span className="bg-manila text-manila-foreground absolute -top-6 left-6 rounded-t-md px-3 py-1 font-mono text-[11px] font-medium tracking-[0.14em] uppercase">
        drive / uploads
      </span>

      <div className="flex items-baseline justify-between gap-4 font-mono">
        <span className="text-sm font-medium">site-photos-2026.zip</span>
        <span className="text-muted-foreground text-xs">48.2 MB · 30 chunks</span>
      </div>

      <div
        aria-hidden="true"
        className="mt-5 grid grid-cols-[repeat(15,minmax(0,1fr))] gap-1"
      >
        {Array.from({ length: TOTAL_CHUNKS }, (_, i) => {
          const lost = frame.phase === "interrupted" && i === INTERRUPT_AT
          return (
            <span
              key={i}
              className={cn(
                "aspect-square rounded-[3px] transition-colors duration-100",
                i < frame.progress
                  ? done
                    ? "bg-success"
                    : "bg-primary"
                  : "bg-border",
                lost && "bg-manila animate-pulse",
              )}
            />
          )
        })}
      </div>

      <figcaption className="mt-5 flex items-center gap-3 font-mono text-xs">
        <span
          className={cn(
            "rounded-full px-2.5 py-0.5 font-medium tracking-wider uppercase",
            phasePill[frame.phase],
          )}
        >
          {phaseLabel[frame.phase]}
        </span>
        <span className="text-muted-foreground">{statusFor(frame)}</span>
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
              Drive Clone keeps every confirmed byte. Lose your connection, close
              the tab, come back tomorrow: your upload resumes exactly where it
              stopped, and nothing you store is visible to anyone else until you
              share it.
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
