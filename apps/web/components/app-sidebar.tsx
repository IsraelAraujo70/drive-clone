"use client"

import { HardDrive, Search, Share2, Trash2 } from "lucide-react"

import { Brand } from "@/components/brand"
import { useCommandMenu } from "@/components/command-menu"
import { Progress } from "@/components/ui/progress"
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar"
import { useAuth } from "@/lib/auth"
import { formatBytes } from "@/lib/format"

const sections = [
  { title: "My Drive", icon: HardDrive, active: true },
  { title: "Shared", icon: Share2, active: false },
  { title: "Trash", icon: Trash2, active: false },
]

export function AppSidebar() {
  const { user } = useAuth()
  const { openMenu } = useCommandMenu()

  const usedPercent = user
    ? Math.min(100, (user.storage_used_bytes / user.storage_quota_bytes) * 100)
    : 0

  return (
    <Sidebar collapsible="icon">
      <SidebarHeader className="p-3 group-data-[collapsible=icon]:p-2">
        <Brand className="group-data-[collapsible=icon]:justify-center" />
      </SidebarHeader>

      <SidebarContent>
        <SidebarGroup>
          <SidebarMenu>
            <SidebarMenuItem>
              <SidebarMenuButton
                tooltip="Search (⌘K)"
                onClick={openMenu}
                className="text-muted-foreground"
              >
                <Search />
                <span>Search files</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
            {sections.map((section) => (
              <SidebarMenuItem key={section.title}>
                <SidebarMenuButton tooltip={section.title} isActive={section.active}>
                  <section.icon />
                  <span>{section.title}</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
            ))}
          </SidebarMenu>
        </SidebarGroup>
      </SidebarContent>

      {user && (
        <SidebarFooter className="group-data-[collapsible=icon]:hidden">
          <div className="flex flex-col gap-2 p-2">
            <strong className="text-sm">
              {formatBytes(user.storage_used_bytes)} of{" "}
              {formatBytes(user.storage_quota_bytes)}
            </strong>
            <Progress value={usedPercent} aria-label="Storage usage" />
            <span className="text-muted-foreground text-xs">Storage used</span>
          </div>
        </SidebarFooter>
      )}
    </Sidebar>
  )
}
