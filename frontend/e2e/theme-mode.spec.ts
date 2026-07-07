import { expect, test } from "@playwright/test"

import { APP_THEME_STORAGE_KEY } from "../lib/theme"

const tokenKey = "drive_clone_token"

async function mockDriveApi(page: import("@playwright/test").Page) {
  await page.route("**/*", async (route) => {
    const request = route.request()
    const url = new URL(request.url())

    if (url.pathname === "/auth/me") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          id: "user-1",
          email: "theme@example.com",
          display_name: "Theme User",
          storage_quota_bytes: 52428800,
          storage_used_bytes: 0,
          created_at: "2026-07-06T12:00:00Z",
        }),
      })
      return
    }

    if (url.pathname === "/drive" && request.resourceType() !== "document") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          parent_folder_id: null,
          breadcrumbs: [],
          folders: [],
          files: [],
        }),
      })
      return
    }

    await route.continue()
  })
}

test("landing follows the user's dark system theme without a theme button", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "dark" })
  await page.goto("/")

  await expect(page.locator("html")).toHaveClass(/dark/)
  await expect(page.getByRole("button", { name: "Switch to light mode" })).toHaveCount(0)
  await expect(page.getByRole("button", { name: "Switch to dark mode" })).toHaveCount(0)
})

test("landing ignores the persisted app theme preference", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark" })
  await page.addInitScript((storageKey) => {
    window.localStorage.setItem(storageKey, "light")
  }, APP_THEME_STORAGE_KEY)

  await page.goto("/")

  await expect(page.locator("html")).toHaveClass(/dark/)
})

test("drive uses and updates the persisted app theme preference", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" })
  await mockDriveApi(page)
  await page.addInitScript(
    ({ appThemeStorageKey, tokenStorageKey }) => {
      window.localStorage.setItem(tokenStorageKey, "theme-token")
      window.localStorage.setItem(appThemeStorageKey, "dark")
    },
    {
      appThemeStorageKey: APP_THEME_STORAGE_KEY,
      tokenStorageKey: tokenKey,
    },
  )

  await page.goto("/drive")

  await expect(page.locator("html")).toHaveClass(/dark/)
  await page.getByRole("button", { name: "Switch to light mode" }).click()
  await expect(page.locator("html")).not.toHaveClass(/dark/)
  await expect(page.locator("html")).toHaveClass(/light/)
  await expect
    .poll(() => page.evaluate((storageKey) => {
      return window.localStorage.getItem(storageKey)
    }, APP_THEME_STORAGE_KEY))
    .toBe("light")
})
