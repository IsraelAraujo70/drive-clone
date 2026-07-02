"use client"

import { useRouter } from "next/navigation"
import { FolderOpen, Search } from "lucide-react"

import { AppSidebar } from "@/components/app-sidebar"
import { CommandMenuProvider, useCommandMenu } from "@/components/command-menu"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty"
import { Kbd, KbdGroup } from "@/components/ui/kbd"
import { Separator } from "@/components/ui/separator"
import { SidebarInset, SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar"
import { type User } from "@/lib/api"
import { useAuth } from "@/lib/auth"
import { formatBytes } from "@/lib/format"
import { useModifierSymbol } from "@/lib/platform"

function HeaderSearch() {
  const { openMenu } = useCommandMenu()
  const modifier = useModifierSymbol()

  return (
    <button
      type="button"
      onClick={openMenu}
      className="text-muted-foreground border-input bg-background hover:bg-accent/40 flex w-full max-w-md items-center gap-2 rounded-lg border px-3 py-2 text-sm transition-colors"
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

function DriveHeader({ user, onLogout }: { user: User; onLogout: () => void }) {
  return (
    <header className="border-border bg-card/80 sticky top-0 z-10 flex items-center gap-3 border-b px-4 py-3 backdrop-blur">
      <SidebarTrigger aria-label="Toggle sidebar (⌘B)" />
      <Separator orientation="vertical" className="mr-1 !h-6" />
      <HeaderSearch />
      <div className="ml-auto flex items-center gap-2">
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

export function DriveShell() {
  const { user, logout } = useAuth()
  const router = useRouter()

  if (!user) {
    return null // RequireAuth guarantees a user; this satisfies the type checker
  }

  async function handleLogout() {
    await logout()
    router.replace("/")
  }

  return (
    <SidebarProvider>
      <CommandMenuProvider>
        <AppSidebar />
        <SidebarInset>
          <DriveHeader user={user} onLogout={handleLogout} />

          <main className="flex flex-col gap-5 p-6 md:p-7">
            <div>
              <h1 className="font-heading text-3xl font-bold tracking-tight">
                My Drive
              </h1>
              <p className="text-muted-foreground">Signed in as {user.email}.</p>
            </div>

            <div className="grid gap-4 sm:grid-cols-3">
              <Card>
                <CardHeader>
                  <CardDescription>Files</CardDescription>
                  <CardTitle className="font-heading text-2xl">0</CardTitle>
                </CardHeader>
              </Card>
              <Card>
                <CardHeader>
                  <CardDescription>Folders</CardDescription>
                  <CardTitle className="font-heading text-2xl">0</CardTitle>
                </CardHeader>
              </Card>
              <Card>
                <CardHeader>
                  <CardDescription>Storage used</CardDescription>
                  <CardTitle className="font-heading text-2xl">
                    {formatBytes(user.storage_used_bytes)}
                  </CardTitle>
                </CardHeader>
              </Card>
            </div>

            <Card className="py-0">
              <Empty>
                <EmptyHeader>
                  <EmptyMedia variant="icon">
                    <FolderOpen />
                  </EmptyMedia>
                  <EmptyTitle>No files yet</EmptyTitle>
                  <EmptyDescription>
                    Uploads arrive in the next milestone.
                  </EmptyDescription>
                </EmptyHeader>
              </Empty>
            </Card>
          </main>
        </SidebarInset>
      </CommandMenuProvider>
    </SidebarProvider>
  )
}
