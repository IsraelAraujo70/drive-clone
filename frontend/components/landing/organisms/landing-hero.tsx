"use client"

import Link from "next/link"

import { UploadFlowCard } from "@/components/landing/molecules/upload-flow-card"
import { Button } from "@/components/ui/button"
import { useAuth } from "@/lib/auth"

export function LandingHero() {
  const { user } = useAuth()
  const primaryHref = user ? "/drive" : "/signup"
  const primaryLabel = user ? "Open your drive" : "Create your drive"

  return (
    <section className="mx-auto grid w-full max-w-6xl items-center gap-14 px-6 pt-20 pb-16 lg:grid-cols-[1.1fr_1fr]">
      <div className="flex flex-col gap-6">
        <p className="flex items-center gap-2 font-mono text-xs tracking-[0.18em] text-muted-foreground uppercase">
          <span aria-hidden="true" className="inline-block size-2 bg-manila" />
          Personal cloud storage · 50 MB free
        </p>
        <h1 className="font-heading text-[clamp(2.75rem,6vw,4.5rem)] leading-[0.98] font-bold tracking-tight">
          Uploads that finish.
          <br />
          Files that stay yours.
        </h1>
        <p className="max-w-[34rem] text-lg leading-relaxed text-muted-foreground">
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
      <UploadFlowCard />
    </section>
  )
}
