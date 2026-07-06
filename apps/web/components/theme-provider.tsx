"use client"

import * as React from "react"
import { Moon, Sun } from "lucide-react"
import { usePathname } from "next/navigation"
import { ThemeProvider as NextThemesProvider, useTheme } from "next-themes"

import { Button } from "@/components/ui/button"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import {
  getNextTheme,
  getThemeStorageKey,
  isAppThemeRoute,
} from "@/lib/theme"

function subscribeMounted() {
  return () => undefined
}

function getMountedSnapshot() {
  return true
}

function getServerMountedSnapshot() {
  return false
}

function ThemeProvider({
  children,
  ...props
}: React.ComponentProps<typeof NextThemesProvider>) {
  const pathname = usePathname()
  const storageKey = getThemeStorageKey(pathname)
  const appThemeRoute = isAppThemeRoute(pathname)

  return (
    <NextThemesProvider
      key={storageKey}
      attribute="class"
      defaultTheme="system"
      enableSystem
      storageKey={storageKey}
      disableTransitionOnChange
      {...props}
    >
      {appThemeRoute && <ThemeHotkey />}
      {children}
    </NextThemesProvider>
  )
}

function isTypingTarget(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) {
    return false
  }

  return (
    target.isContentEditable ||
    target.tagName === "INPUT" ||
    target.tagName === "TEXTAREA" ||
    target.tagName === "SELECT"
  )
}

function ThemeHotkey() {
  const { resolvedTheme, setTheme } = useTheme()

  React.useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.defaultPrevented || event.repeat) {
        return
      }

      if (event.metaKey || event.ctrlKey || event.altKey) {
        return
      }

      if (event.key.toLowerCase() !== "d") {
        return
      }

      if (isTypingTarget(event.target)) {
        return
      }

      setTheme(getNextTheme(resolvedTheme))
    }

    window.addEventListener("keydown", onKeyDown)

    return () => {
      window.removeEventListener("keydown", onKeyDown)
    }
  }, [resolvedTheme, setTheme])

  return null
}

function ThemeToggleButton() {
  const mounted = React.useSyncExternalStore(
    subscribeMounted,
    getMountedSnapshot,
    getServerMountedSnapshot,
  )
  const { resolvedTheme, setTheme } = useTheme()

  const nextTheme = getNextTheme(resolvedTheme)
  const label =
    nextTheme === "dark" ? "Switch to dark mode" : "Switch to light mode"
  const Icon = resolvedTheme === "dark" ? Sun : Moon

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          type="button"
          variant="outline"
          size="icon"
          aria-label={label}
          disabled={!mounted}
          onClick={() => setTheme(nextTheme)}
        >
          <Icon aria-hidden="true" />
        </Button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  )
}

export { ThemeProvider, ThemeToggleButton }
