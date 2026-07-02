import Link from "next/link"

import { cn } from "@/lib/utils"

export function Brand({ className }: { className?: string }) {
  return (
    <Link
      href="/"
      className={cn("text-foreground flex items-center gap-3 font-semibold", className)}
    >
      <span className="bg-primary text-primary-foreground font-heading grid size-9 shrink-0 place-items-center rounded-lg text-lg font-bold group-data-[collapsible=icon]:size-8">
        D
      </span>
      {/* Hidden when the sidebar collapses to icons; inert outside a sidebar group. */}
      <span className="group-data-[collapsible=icon]:hidden">Drive Clone</span>
    </Link>
  )
}
