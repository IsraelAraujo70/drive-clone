import { describe, expect, it } from "vitest"
import {
  APP_THEME_STORAGE_KEY,
  PUBLIC_THEME_STORAGE_KEY,
  getNextTheme,
  getThemeStorageKey,
  isAppThemeRoute,
} from "./theme"

describe("theme scope", () => {
  it("keeps public pages on the public theme storage key", () => {
    expect(getThemeStorageKey("/")).toBe(PUBLIC_THEME_STORAGE_KEY)
    expect(getThemeStorageKey("/login")).toBe(PUBLIC_THEME_STORAGE_KEY)
    expect(getThemeStorageKey("/signup")).toBe(PUBLIC_THEME_STORAGE_KEY)
  })

  it("uses the app theme storage key inside the drive", () => {
    expect(getThemeStorageKey("/drive")).toBe(APP_THEME_STORAGE_KEY)
    expect(getThemeStorageKey("/drive/nested")).toBe(APP_THEME_STORAGE_KEY)
  })

  it("identifies app theme routes", () => {
    expect(isAppThemeRoute("/drive")).toBe(true)
    expect(isAppThemeRoute("/drive/folder")).toBe(true)
    expect(isAppThemeRoute("/login")).toBe(false)
  })

  it("returns the next explicit theme", () => {
    expect(getNextTheme("dark")).toBe("light")
    expect(getNextTheme("light")).toBe("dark")
    expect(getNextTheme(undefined)).toBe("dark")
  })
})
