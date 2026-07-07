"use client"

import { useEffect, useState } from "react"

import { Brand } from "@/components/atoms/brand"
import { PublicShareCard } from "@/components/share/organisms/public-share-card"
import { loadPublicShareLink, type PublicShareLinkState } from "@/lib/shareLink"

type LoadState = { status: "loading" } | PublicShareLinkState

export function PublicSharePage({ token }: { token: string }) {
  const [state, setState] = useState<LoadState>({ status: "loading" })

  useEffect(() => {
    let active = true
    void loadPublicShareLink(token).then((next) => {
      if (active) {
        setState(next)
      }
    })
    return () => {
      active = false
    }
  }, [token])

  return (
    <div className="flex min-h-svh flex-col items-center justify-center gap-8 p-6">
      <Brand />
      <PublicShareCard state={state} />
    </div>
  )
}
