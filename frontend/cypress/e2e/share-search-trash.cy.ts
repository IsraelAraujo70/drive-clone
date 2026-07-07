function uniqueId() {
  return `${Date.now()}-${Cypress._.random(100000, 999999)}`
}

describe("share, search, and trash", () => {
  it("keeps access private, grants shares, scopes search, and purges from trash", () => {
    const id = uniqueId()
    const ownerEmail = `owner-${id}@example.com`
    const granteeEmail = `grantee-${id}@example.com`
    const filename = `shared-search-trash-${id}.txt`
    let ownerToken = ""
    let granteeToken = ""
    let fileId = ""

    cy.signupByApi(ownerEmail).then((owner) => {
      ownerToken = owner.token
    })
    cy.signupByApi(granteeEmail).then((grantee) => {
      granteeToken = grantee.token
    })
    cy.then(() =>
      cy
        .seedCompletedFile(ownerToken, {
          filename,
          contentType: "text/plain",
          bytes: `private file ${id}`,
        })
        .then((seeded) => {
          fileId = seeded.fileId
        })
    )

    cy.then(() => {
      cy.api("GET", `/files/${fileId}/download`, {
        token: granteeToken,
        failOnStatusCode: false,
      })
        .its("status")
        .should("be.oneOf", [401, 403, 404])
    })

    cy.then(() => cy.authenticatedVisit("/drive", ownerToken))
    cy.contains('[data-cy="file-item"]', filename, { timeout: 15000 })
      .find('[data-cy="share-action"]')
      .click()
    cy.get('[data-cy="share-email-input"]').type(granteeEmail)
    cy.get('[data-cy="share-submit"]').click()
    cy.contains(granteeEmail, { timeout: 15000 }).should("be.visible")
    cy.get("body").type("{esc}")

    cy.get('[data-cy="search-trigger"]').first().click()
    cy.get('[data-cy="search-input"]').type(filename)
    cy.contains(filename, { timeout: 15000 }).should("be.visible")
    cy.get("body").type("{esc}")

    cy.then(() => cy.authenticatedVisit("/drive", granteeToken))
    cy.get('[data-cy="nav-shared"]').click()
    cy.contains('[data-cy="file-item"]', filename, { timeout: 15000 }).should(
      "be.visible"
    )
    cy.get('[data-cy="search-trigger"]').first().click()
    cy.get('[data-cy="search-input"]').type(filename)
    cy.contains(filename, { timeout: 15000 }).should("be.visible")
    cy.get("body").type("{esc}")

    cy.then(() => cy.authenticatedVisit("/drive", ownerToken))
    cy.contains('[data-cy="file-item"]', filename, { timeout: 15000 })
      .contains("button", "Delete")
      .click()
    cy.contains('[data-cy="file-item"]', filename).should("not.exist")

    cy.then(() => cy.authenticatedVisit("/drive", granteeToken))
    cy.get('[data-cy="nav-shared"]').click()
    cy.contains('[data-cy="file-item"]', filename).should("not.exist")
    cy.api("GET", `/files/${fileId}/download`, {
      token: granteeToken,
      failOnStatusCode: false,
    })
      .its("status")
      .should("be.oneOf", [401, 403, 404])

    cy.then(() => cy.authenticatedVisit("/drive", ownerToken))
    cy.get('[data-cy="nav-trash"]').click()
    cy.contains('[data-cy="file-item"]', filename, { timeout: 15000 })
      .contains("button", "Restore")
      .click()
    cy.contains('[data-cy="file-item"]', filename).should("not.exist")

    cy.then(() => cy.authenticatedVisit("/drive", ownerToken))
    cy.contains('[data-cy="file-item"]', filename, { timeout: 15000 })
      .contains("button", "Delete")
      .click()
    cy.get('[data-cy="nav-trash"]').click()
    cy.contains('[data-cy="file-item"]', filename, { timeout: 15000 }).within(
      () => {
        cy.on("window:confirm", () => true)
        cy.contains("button", "Force delete").click()
      }
    )
    cy.contains('[data-cy="file-item"]', filename).should("not.exist")
    cy.api("GET", `/files/${fileId}/download`, {
      token: ownerToken,
      failOnStatusCode: false,
    })
      .its("status")
      .should("be.oneOf", [401, 403, 404])
  })
})

export {}
