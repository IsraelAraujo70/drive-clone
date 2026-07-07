import Link from "next/link"
import Image from "next/image"

import { cn } from "@/lib/utils"

export function Brand({ className }: { className?: string }) {
  return (
    <Link
      href="/"
      className={cn(
        "flex items-center gap-3 font-semibold text-foreground",
        className
      )}
    >
      <Image
        src="/logo.png"
        alt=""
        width={36}
        height={36}
        className="size-9 shrink-0 rounded-lg group-data-[collapsible=icon]:size-8"
        priority
      />
      {/* Hidden when the sidebar collapses to icons; inert outside a sidebar group. */}
      <span className="group-data-[collapsible=icon]:hidden">Drive Clone</span>
    </Link>
  )
}
