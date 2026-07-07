"use client"

import { useSyncExternalStore } from "react"

export function isMacPlatform(platform: string): boolean {
  return /mac|iphone|ipad|ipod/i.test(platform)
}

// Platform never changes at runtime, so there is nothing to subscribe to.
function noopSubscribe() {
  return () => {}
}

/**
 * Whether the client runs on an Apple platform. Returns false during SSR and
 * the first client render (so markup matches), then the real value on the client.
 */
export function useIsMac(): boolean {
  return useSyncExternalStore(
    noopSubscribe,
    () => isMacPlatform(navigator.platform),
    () => false,
  )
}

/** The primary modifier symbol for the current platform: ⌘ on macOS, Ctrl elsewhere. */
export function useModifierSymbol(): string {
  return useIsMac() ? "⌘" : "Ctrl"
}
