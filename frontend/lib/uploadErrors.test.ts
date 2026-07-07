import { describe, expect, it } from "vitest"

import { ApiError, UploadPartError } from "./api"
import { getUploadErrorMessage } from "./uploadErrors"

describe("getUploadErrorMessage", () => {
  it("names the failed part for a network/storage error", () => {
    expect(getUploadErrorMessage(new UploadPartError(3, "network"))).toBe(
      "Couldn't upload part 3. Check your connection and resume the upload."
    )
    expect(getUploadErrorMessage(new UploadPartError(7, "http", 502))).toBe(
      "Couldn't upload part 7. Check your connection and resume the upload."
    )
  })

  it("treats expired-session API codes as a restart prompt", () => {
    expect(
      getUploadErrorMessage(
        new ApiError(409, "invalid_file_state", "File is not in the expected state")
      )
    ).toBe("This upload expired. Start it again.")
    expect(
      getUploadErrorMessage(new ApiError(404, "file_not_found", "File was not found"))
    ).toBe("This upload expired. Start it again.")
  })

  it("uses friendly copy for known API codes", () => {
    expect(
      getUploadErrorMessage(new ApiError(409, "quota_exceeded", "no room"))
    ).toBe("Not enough storage is available for this upload.")
    expect(
      getUploadErrorMessage(new ApiError(401, "unauthorized", "bad token"))
    ).toBe("Your session expired. Log in again to keep uploading.")
  })

  it("falls back to code:message for unknown API errors", () => {
    expect(
      getUploadErrorMessage(new ApiError(502, "storage_error", "bad gateway"))
    ).toBe("storage_error: bad gateway")
  })

  it("falls back to a generic message for non-Error values", () => {
    expect(getUploadErrorMessage("boom")).toBe("Something went wrong. Try again.")
    expect(getUploadErrorMessage(new Error("plain"))).toBe("plain")
  })
})
