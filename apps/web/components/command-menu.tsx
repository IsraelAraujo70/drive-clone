"use client"

import {
  createContext,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react"
import { useRouter } from "next/navigation"
import { AlertCircle, FileIcon, HardDrive, LogOut, Search, Share2, Trash2 } from "lucide-react"

import {
  Command,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from "@/components/ui/command"
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog"
import { Kbd, KbdGroup } from "@/components/ui/kbd"
import { Spinner } from "@/components/ui/spinner"
import { api, type SearchFileResult } from "@/lib/api"
import { useAuth } from "@/lib/auth"

type CommandMenuView = "my-drive" | "shared-with-me" | "trash"

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

export function CommandMenuProvider({
  children,
  onViewChange,
  onDownload,
}: {
  children: ReactNode
  onViewChange?: (view: CommandMenuView) => void
  onDownload?: (fileId: string) => void | Promise<void>
}) {
  const router = useRouter()
  const { logout, token } = useAuth()
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState("")
  const [results, setResults] = useState<SearchFileResult[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

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

  useEffect(() => {
    const trimmed = query.trim()
    if (!open || !trimmed || !token) {
      return
    }

    let cancelled = false
    const timer = window.setTimeout(() => {
      if (cancelled) {
        return
      }
      setLoading(true)
      void api
        .searchFiles(token, trimmed)
        .then((response) => {
          if (!cancelled) {
            setResults(response.files)
          }
        })
        .catch((caught: unknown) => {
          if (!cancelled) {
            setResults([])
            setError(caught instanceof Error ? caught.message : "Search failed")
          }
        })
        .finally(() => {
          if (!cancelled) {
            setLoading(false)
          }
        })
    }, 200)

    return () => {
      cancelled = true
      window.clearTimeout(timer)
    }
  }, [open, query, token])

  const resetSearch = () => {
    setQuery("")
    setResults([])
    setError(null)
    setLoading(false)
  }

  const handleOpenChange = (nextOpen: boolean) => {
    setOpen(nextOpen)
    if (!nextOpen) {
      resetSearch()
    }
  }

  const handleQueryChange = (value: string) => {
    setQuery(value)
    setError(null)
    if (value.trim()) {
      setLoading(true)
    } else {
      setResults([])
      setLoading(false)
    }
  }

  const runCommand = (action: () => void | Promise<void>) => {
    setOpen(false)
    resetSearch()
    void action()
  }

  const openMenu = () => setOpen(true)

  const goToView = (view: CommandMenuView) => {
    if (onViewChange) {
      onViewChange(view)
      return
    }
    router.push("/drive")
  }

  const searchActive = query.trim().length > 0

  return (
    <CommandMenuContext.Provider value={{ open, setOpen, openMenu }}>
      {children}
      <Dialog open={open} onOpenChange={handleOpenChange}>
        <DialogTitle className="sr-only">Search and commands</DialogTitle>
        <DialogDescription className="sr-only">
          Search your drive or jump to a section.
        </DialogDescription>
        <DialogContent
          showCloseButton={false}
          className="top-1/4 translate-y-0 overflow-hidden p-0 sm:max-w-lg"
        >
          <Command shouldFilter={false}>
            <CommandInput
              placeholder="Search files or jump to…"
              value={query}
              onValueChange={handleQueryChange}
            />
            <CommandList>
              {searchActive && (
                <CommandGroup heading="Results">
                  {loading ? (
                    <CommandItem disabled>
                      <Spinner />
                      Searching files…
                    </CommandItem>
                  ) : error ? (
                    <CommandItem disabled>
                      <AlertCircle />
                      {error}
                    </CommandItem>
                  ) : results.length === 0 ? (
                    <CommandItem disabled>
                      <Search />
                      No matching files
                    </CommandItem>
                  ) : (
                    results.map((result) => (
                      <CommandItem
                        key={`${result.access}-${result.file.id}`}
                        value={`${result.file.filename} ${result.owner?.display_name ?? ""} ${result.owner?.email ?? ""}`}
                        onSelect={() =>
                          runCommand(() => onDownload?.(result.file.id))
                        }
                      >
                        <FileIcon />
                        <div className="flex min-w-0 flex-col">
                          <span className="truncate">{result.file.filename}</span>
                          <span className="text-muted-foreground truncate text-xs">
                            {result.access === "owned"
                              ? "My Drive"
                              : `Shared by ${result.owner?.display_name ?? "Unknown"}`}
                          </span>
                        </div>
                      </CommandItem>
                    ))
                  )}
                </CommandGroup>
              )}
              {searchActive && <CommandSeparator />}
              <CommandGroup heading="Go to">
                <CommandItem
                  onSelect={() => runCommand(() => goToView("my-drive"))}
                >
                  <HardDrive />
                  My Drive
                </CommandItem>
                <CommandItem
                  onSelect={() => runCommand(() => goToView("shared-with-me"))}
                >
                  <Share2 />
                  Shared
                </CommandItem>
                <CommandItem
                  onSelect={() => runCommand(() => goToView("trash"))}
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
