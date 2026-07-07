export const APP_THEME_STORAGE_KEY = "drive_clone_app_theme"
export const PUBLIC_THEME_STORAGE_KEY = "drive_clone_public_theme"

export function isAppThemeRoute(pathname: string | null | undefined) {
  return pathname === "/drive" || pathname?.startsWith("/drive/")
}

export function getThemeStorageKey(pathname: string | null | undefined) {
  return isAppThemeRoute(pathname)
    ? APP_THEME_STORAGE_KEY
    : PUBLIC_THEME_STORAGE_KEY
}

export function getNextTheme(resolvedTheme: string | undefined) {
  return resolvedTheme === "dark" ? "light" : "dark"
}
