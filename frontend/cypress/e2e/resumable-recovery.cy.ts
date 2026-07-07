const pendingUploadsKey = "drive_clone_resumable_uploads_v1"
const partSizeBytes = 5 * 1024 * 1024
const contentType = "application/octet-stream"

function uniqueId() {
  return `${Date.now()}-${Cypress._.random(100000, 999999)}`
}

describe("resumable upload recovery", () => {
  it("resumes a real multipart session and finalizes the completed file", () => {
    const id = uniqueId()
    const email = `resume-${id}@example.com`
    const filename = `resume-demo-${id}.bin`
    const sizeBytes = partSizeBytes + 17
    const lastModified = Date.UTC(2026, 0, 1)
    const bytes = Cypress.Buffer.alloc(sizeBytes, 7)
    let token = ""
    let fileId = ""
    let expiresAt = ""

    cy.signupByApi(email).then((auth) =>
      cy
        .createResumableSession(auth.token, {
          filename,
          contentType,
          sizeBytes,
          partSizeBytes,
        })
        .then((session) => {
          token = auth.token
          fileId = session.file_id
          expiresAt = session.expires_at
        })
    )
    cy.then(() => {
      return cy.api<{ upload_url: string }>(
        "POST",
        `/files/uploads/${fileId}/parts`,
        {
          token,
          body: { part_number: 1 },
        }
      )
    })
      .then((signed) =>
        cy.request({
          method: "PUT",
          url: signed.body.upload_url,
          body: Cypress.Buffer.from(bytes.subarray(0, partSizeBytes)).toString(
            "binary"
          ),
          encoding: "binary",
        })
      )
      .then((uploaded) => {
        const etag = String(uploaded.headers.etag)
        return cy.api("POST", `/files/uploads/${fileId}/parts/1`, {
          token,
          body: {
            size_bytes: partSizeBytes,
            etag,
          },
        })
      })

    cy.then(() => {
      const key = [filename, sizeBytes, lastModified, contentType, "root"].join(
        ":"
      )

      cy.visit("/drive", {
        onBeforeLoad(win) {
          win.localStorage.setItem("drive_clone_token", token)
          win.localStorage.setItem(
            pendingUploadsKey,
            JSON.stringify({
              [key]: {
                file_id: fileId,
                filename,
                size_bytes: sizeBytes,
                last_modified: lastModified,
                content_type: contentType,
                parent_folder_id: null,
                expires_at: expiresAt,
              },
            })
          )
        },
      })
    })

    cy.get('[data-cy="pending-upload-card"]', { timeout: 15000 }).should(
      "contain",
      filename
    )
    cy.get('[data-cy="resume-upload"]').click()
    cy.get('[data-cy="resume-upload-input"]').selectFile(
      {
        contents: bytes,
        fileName: filename,
        mimeType: contentType,
        lastModified,
      },
      { force: true }
    )

    cy.get('[data-cy="pending-upload-card"]', { timeout: 30000 }).should(
      "not.exist"
    )
    cy.contains('[data-cy="file-item"]', filename, { timeout: 15000 }).should(
      "be.visible"
    )

    cy.then(() =>
      cy
        .api<{ download_url: string }>("GET", `/files/${fileId}/download`, {
          token,
        })
        .then((download) =>
          cy
            .request({ url: download.body.download_url, encoding: "binary" })
            .then((response) => {
              const downloaded = Cypress.Buffer.from(response.body, "binary")
              expect(downloaded.equals(bytes)).to.eq(true)
            })
        )
    )
  })
})

export {}
