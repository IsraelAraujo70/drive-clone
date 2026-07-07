"use client"

import { Search } from "lucide-react"

import { ThemeToggleButton } from "@/components/atoms/theme-toggle-button"
import { useCommandMenu } from "@/components/command/organisms/command-menu-provider"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Kbd, KbdGroup } from "@/components/ui/kbd"
import { Separator } from "@/components/ui/separator"
import { SidebarTrigger } from "@/components/ui/sidebar"
import type { User } from "@/lib/api"
import { useModifierSymbol } from "@/lib/platform"

function HeaderSearch() {
  const { openMenu } = useCommandMenu()
  const modifier = useModifierSymbol()

  return (
    <button
      type="button"
      data-cy="search-trigger"
      onClick={openMenu}
      className="flex w-full max-w-md items-center gap-2 rounded-lg border border-input bg-background px-3 py-2 text-sm text-muted-foreground transition-colors hover:bg-accent/40"
    >
      <Search aria-hidden="true" className="size-4" />
      <span className="flex-1 text-left">Search files</span>
      <KbdGroup>
        <Kbd>{modifier}</Kbd>
        <Kbd>K</Kbd>
      </KbdGroup>
    </button>
  )
}

export function DriveHeader({
  user,
  onLogout,
}: {
  user: User
  onLogout: () => void
}) {
  return (
    <header className="sticky top-0 z-10 flex items-center gap-3 border-b border-border bg-card/80 px-4 py-3 backdrop-blur">
      <SidebarTrigger aria-label="Toggle sidebar (⌘B)" />
      <Separator orientation="vertical" className="mr-1 !h-6" />
      <HeaderSearch />
      <div className="ml-auto flex items-center gap-2">
        <ThemeToggleButton />
        <Badge variant="outline" title={user.email}>
          {user.display_name}
        </Badge>
        <Button variant="outline" onClick={onLogout}>
          Log out
        </Button>
      </div>
    </header>
  )
}
