"use client"

import Link from "next/link"

import { Brand } from "@/components/atoms/brand"
import { Button } from "@/components/ui/button"
import { useAuth } from "@/lib/auth"

export function LandingHeader() {
  const { user } = useAuth()

  return (
    <header className="border-b border-border bg-card">
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
  )
}
