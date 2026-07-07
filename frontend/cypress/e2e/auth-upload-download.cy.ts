const password = "Password123!"

function uniqueId() {
  return `${Date.now()}-${Cypress._.random(100000, 999999)}`
}

describe("auth, upload, and download", () => {
  it("signs up, uploads through the real stack, and downloads exact bytes", () => {
    const id = uniqueId()
    const email = `upload-${id}@example.com`
    const filename = `hello-${id}.txt`
    const content = `Drive Clone Cypress upload ${id}\n`

    cy.visit("/signup")
    cy.get('[data-cy="signup-display-name"]').type("Upload User")
    cy.get('[data-cy="signup-email"]').type(email)
    cy.get('input[data-cy="signup-password"]').clear().type(password)
    cy.contains("Strong").should("be.visible")
    cy.get('input[data-cy="signup-confirm-password"]').clear().type(password)
    cy.get('[data-cy="signup-submit"]').should("not.be.disabled").click()

    cy.location("pathname", { timeout: 15000 }).should("eq", "/drive")

    cy.intercept("POST", "**/files/uploads/*/finalize").as("finalizeUpload")
    cy.intercept("GET", "**/files/*/download").as("createDownload")

    cy.get('[data-cy="drive-upload-input"]').selectFile(
      {
        contents: Cypress.Buffer.from(content),
        fileName: filename,
        mimeType: "text/plain",
      },
      { force: true }
    )

    cy.get('[data-cy="upload-progress"]', { timeout: 10000 }).should(
      "contain",
      filename
    )
    cy.wait("@finalizeUpload", { timeout: 30000 })
      .its("response.statusCode")
      .should("eq", 200)
    cy.contains('[data-cy="file-item"]', filename, { timeout: 15000 }).should(
      "be.visible"
    )

    cy.window().then((win) => {
      cy.stub(win, "open").as("downloadOpen")
    })
    cy.contains('[data-cy="file-item"]', filename)
      .contains("button", "Download")
      .click()

    cy.wait("@createDownload").then((interception) => {
      expect(interception.response?.statusCode).to.eq(200)
      const downloadUrl = interception.response?.body.download_url as string
      cy.get("@downloadOpen").should(
        "have.been.calledWith",
        downloadUrl,
        "_self"
      )
      cy.request({ url: downloadUrl, encoding: "binary" }).then((response) => {
        const downloaded = Cypress.Buffer.from(response.body, "binary")
        expect(downloaded.equals(Cypress.Buffer.from(content))).to.eq(true)
      })
    })
  })
})

export {}
