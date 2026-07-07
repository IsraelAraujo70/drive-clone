const tokenKey = "drive_clone_token"
const appThemeKey = "drive_clone_app_theme"
const publicThemeKey = "drive_clone_public_theme"

function apiUrl(path: string) {
  return `${String(Cypress.env("apiBaseUrl") ?? "http://localhost:8080")}${path}`
}

function mockDriveApi() {
  cy.intercept("GET", apiUrl("/auth/me"), {
    id: "theme-user",
    email: "theme@example.com",
    display_name: "Theme User",
    storage_quota_bytes: 52428800,
    storage_used_bytes: 0,
    created_at: "2026-01-01T00:00:00Z",
  })
  cy.intercept("GET", apiUrl("/drive"), {
    parent_folder_id: null,
    breadcrumbs: [],
    folders: [],
    files: [],
  })
  cy.intercept("GET", apiUrl("/files/uploads/pending"), { uploads: [] })
}

function systemTheme(dark: boolean) {
  return {
    onBeforeLoad(win: Window) {
      win.matchMedia = (query: string) =>
        ({
          matches: query.includes("prefers-color-scheme: dark") ? dark : false,
          media: query,
          onchange: null,
          addEventListener: () => undefined,
          removeEventListener: () => undefined,
          addListener: () => undefined,
          removeListener: () => undefined,
          dispatchEvent: () => false,
        }) as MediaQueryList
    },
  }
}

describe("theme mode", () => {
  it("uses the dark system theme on the landing page", () => {
    cy.visit("/", systemTheme(true))

    cy.get("html").should("have.class", "dark")
    cy.get('button[aria-label*="Switch to"]').should("not.exist")
  })

  it("ignores the persisted app theme preference on the landing page", () => {
    cy.visit("/", {
      ...systemTheme(true),
      onBeforeLoad(win) {
        systemTheme(true).onBeforeLoad(win)
        win.localStorage.setItem(appThemeKey, "light")
        win.localStorage.removeItem(publicThemeKey)
      },
    })

    cy.get("html").should("have.class", "dark")
    cy.get('button[aria-label*="Switch to"]').should("not.exist")
  })

  it("uses and updates the persisted app theme preference on drive pages", () => {
    mockDriveApi()

    cy.visit("/drive", {
      ...systemTheme(false),
      onBeforeLoad(win) {
        systemTheme(false).onBeforeLoad(win)
        win.localStorage.setItem(tokenKey, "theme-token")
        win.localStorage.setItem(appThemeKey, "dark")
      },
    })

    cy.get("html").should("have.class", "dark")
    cy.get('button[aria-label="Switch to light mode"]').click()
    cy.get("html").should("have.class", "light")
    cy.window()
      .its("localStorage")
      .invoke("getItem", appThemeKey)
      .should("eq", "light")
  })
})

export {}
