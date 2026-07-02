"use client"

import {
  createContext,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react"
import { useRouter } from "next/navigation"
import { HardDrive, LogOut, Search, Share2, Trash2 } from "lucide-react"

import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from "@/components/ui/command"
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog"
import { Kbd, KbdGroup } from "@/components/ui/kbd"
import { useAuth } from "@/lib/auth"

type CommandMenuContextValue = {
  open: boolean
  setOpen: (open: boolean) => void
  openMenu: () => void
}

const CommandMenuContext = createContext<CommandMenuContextValue | null>(null)

export function useCommandMenu(): CommandMenuContextValue {
  const context = useContext(CommandMenuContext)
  if (!context) {
    throw new Error("useCommandMenu must be used inside CommandMenuProvider")
  }
  return context
}

export function CommandMenuProvider({ children }: { children: ReactNode }) {
  const router = useRouter()
  const { logout } = useAuth()
  const [open, setOpen] = useState(false)

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key.toLowerCase() === "k" && (event.metaKey || event.ctrlKey)) {
        event.preventDefault()
        setOpen((current) => !current)
      }
    }
    window.addEventListener("keydown", onKeyDown)
    return () => window.removeEventListener("keydown", onKeyDown)
  }, [])

  const runCommand = (action: () => void) => {
    setOpen(false)
    action()
  }

  const openMenu = () => setOpen(true)

  return (
    <CommandMenuContext.Provider value={{ open, setOpen, openMenu }}>
      {children}
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogTitle className="sr-only">Search and commands</DialogTitle>
        <DialogDescription className="sr-only">
          Search your drive or jump to a section.
        </DialogDescription>
        <DialogContent
          showCloseButton={false}
          className="top-1/4 translate-y-0 overflow-hidden p-0 sm:max-w-lg"
        >
          <Command>
            <CommandInput placeholder="Search files or jump to…" />
            <CommandList>
              <CommandEmpty>No results found.</CommandEmpty>
              <CommandGroup heading="Search">
                <CommandItem
                  onSelect={() => runCommand(() => router.push("/drive"))}
                >
                  <Search />
                  Search all files
                </CommandItem>
              </CommandGroup>
              <CommandSeparator />
              <CommandGroup heading="Go to">
                <CommandItem
                  onSelect={() => runCommand(() => router.push("/drive"))}
                >
                  <HardDrive />
                  My Drive
                </CommandItem>
                <CommandItem
                  onSelect={() => runCommand(() => router.push("/drive"))}
                >
                  <Share2 />
                  Shared
                </CommandItem>
                <CommandItem
                  onSelect={() => runCommand(() => router.push("/drive"))}
                >
                  <Trash2 />
                  Trash
                </CommandItem>
              </CommandGroup>
              <CommandSeparator />
              <CommandGroup heading="Account">
                <CommandItem
                  onSelect={() =>
                    runCommand(() => {
                      void logout().then(() => router.replace("/"))
                    })
                  }
                >
                  <LogOut />
                  Log out
                </CommandItem>
              </CommandGroup>
            </CommandList>
            <div className="text-muted-foreground flex items-center justify-end gap-3 border-t px-3 py-2 text-xs">
              <span className="flex items-center gap-1.5">
                <KbdGroup>
                  <Kbd>↑</Kbd>
                  <Kbd>↓</Kbd>
                </KbdGroup>
                Navigate
              </span>
              <span className="flex items-center gap-1.5">
                <Kbd>↵</Kbd>
                Open
              </span>
              <span className="flex items-center gap-1.5">
                <Kbd>Esc</Kbd>
                Close
              </span>
            </div>
          </Command>
        </DialogContent>
      </Dialog>
    </CommandMenuContext.Provider>
  )
}
