"use client"

import Link from "next/link"

import { Button } from "@/components/ui/button"
import { Progress } from "@/components/ui/progress"
import { useAuth } from "@/lib/auth"

export function QuotaSection() {
  const { user } = useAuth()
  const primaryHref = user ? "/drive" : "/signup"
  const primaryLabel = user ? "Open your drive" : "Create your drive"

  return (
    <section className="border-t border-border bg-card">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-8 px-6 py-16 lg:flex-row lg:items-center lg:justify-between">
        <div className="flex max-w-md flex-col gap-3">
          <h2 className="font-heading text-3xl font-bold tracking-tight">
            50 MB free, counted in bytes.
          </h2>
          <p className="text-muted-foreground">
            Every account starts with the full quota. No credit card, no trial
            clock.
          </p>
        </div>
        <div className="flex w-full max-w-sm flex-col gap-2">
          <Progress value={100} aria-label="Free storage included" />
          <span className="font-mono text-xs text-muted-foreground">
            storage_quota_bytes = 52,428,800
          </span>
        </div>
        <Button size="lg" asChild>
          <Link href={primaryHref}>{primaryLabel}</Link>
        </Button>
      </div>
    </section>
  )
}
