import { describe, expect, it } from "vitest"
import { ApiError } from "./api"
import { getApiErrorMessage, getErrorMessage, getShareErrorMessage } from "./shareErrors"

describe("share error copy", () => {
  it("turns user_not_found into an actionable sharing message", () => {
    const error = new ApiError(404, "user_not_found", "User was not found")

    expect(getShareErrorMessage(error, "missing@example.com")).toBe(
      "No account uses missing@example.com. Ask them to sign up first, then share this file again.",
    )
  })

  it("keeps non-sharing ApiError details for diagnostics", () => {
    const error = new ApiError(409, "file_not_complete", "File is still uploading")

    expect(getShareErrorMessage(error, "friend@example.com")).toBe(
      "file_not_complete: File is still uploading",
    )
  })

  it("keeps the generic fallback for unknown failures", () => {
    expect(getErrorMessage("boom")).toBe("Something went wrong. Try again.")
    expect(getApiErrorMessage(new Error("Network failed"))).toBe("Network failed")
  })
})
