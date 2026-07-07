import type { ReactNode } from "react"

import { Brand } from "@/components/atoms/brand"

export function AuthPageShell({ children }: { children: ReactNode }) {
  return (
    <div className="flex min-h-svh flex-col items-center justify-center gap-8 p-6">
      <Brand />
      {children}
    </div>
  )
}
