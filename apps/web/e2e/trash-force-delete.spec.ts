import { expect, test } from "@playwright/test"

const tokenKey = "drive_clone_token"

test("trash force delete confirms and calls the purge endpoint", async ({ page }) => {
  let purgeCalls = 0

  await page.route("**/*", async (route) => {
    const request = route.request()
    const url = new URL(request.url())

    if (url.pathname === "/auth/me") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          id: "user-1",
          email: "trash@example.com",
          display_name: "Trash User",
          storage_quota_bytes: 52428800,
          storage_used_bytes: purgeCalls > 0 ? 0 : 12,
          created_at: "2026-07-06T12:00:00Z",
        }),
      })
      return
    }

    if (url.pathname === "/files/uploads/pending") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ uploads: [] }),
      })
      return
    }

    if (url.pathname === "/drive/trash") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          parent_folder_id: null,
          breadcrumbs: [],
          folders: [],
          files:
            purgeCalls > 0
              ? []
              : [
                  {
                    id: "file-1",
                    filename: "old-report.pdf",
                    parent_folder_id: null,
                    content_type: "application/pdf",
                    size_bytes: 12,
                    checksum_sha256: null,
                    object_key: "objects/file-1",
                    state: "complete",
                    created_at: "2026-07-02T12:00:00Z",
                    updated_at: "2026-07-03T12:00:00Z",
                    completed_at: "2026-07-02T12:01:00Z",
                    deleted_at: "2026-07-03T12:00:00Z",
                  },
                ],
        }),
      })
      return
    }

    if (url.pathname === "/files/file-1/purge") {
      purgeCalls += 1
      expect(request.method()).toBe("DELETE")
      await route.fulfill({ status: 204 })
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

  await page.addInitScript((storageKey) => {
    window.localStorage.setItem(storageKey, "trash-token")
  }, tokenKey)

  page.on("dialog", async (dialog) => {
    expect(dialog.message()).toContain("Permanently delete old-report.pdf")
    await dialog.accept()
  })

  await page.goto("/drive")
  await page.getByRole("button", { name: "Trash" }).click()
  await expect(page.getByText("old-report.pdf")).toBeVisible()

  await page.getByRole("button", { name: "Force delete" }).click()

  await expect(page.getByText("Trash is empty")).toBeVisible()
  expect(purgeCalls).toBe(1)
})
