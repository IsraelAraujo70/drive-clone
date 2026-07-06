import { afterEach, describe, expect, it, vi } from "vitest"
import { ApiError } from "./api"
import { getShareLinkErrorMessage } from "./shareErrors"
import { loadPublicShareLink } from "./shareLink"

function jsonResponse(status: number, body: unknown) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  })
}

afterEach(() => {
  vi.unstubAllGlobals()
})

describe("public share link error copy", () => {
  it("turns any 404 into a single uniform unavailable message", () => {
    const error = new ApiError(404, "file_not_found", "File was not found")
    expect(getShareLinkErrorMessage(error)).toBe(
      "This link is no longer available. It may have been revoked, expired, or the file was deleted.",
    )
  })

  it("falls back to a generic message for non-404 failures", () => {
    expect(getShareLinkErrorMessage(new Error("Network failed"))).toBe(
      "Network failed",
    )
    expect(getShareLinkErrorMessage("boom")).toBe(
      "Something went wrong. Try again.",
    )
  })
})

describe("loadPublicShareLink", () => {
  it("returns file metadata when the token resolves", async () => {
    const file = {
      filename: "report.pdf",
      size_bytes: 42,
      content_type: "application/pdf",
      download_url: "https://storage.example/download",
    }
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(200, file))
    vi.stubGlobal("fetch", fetchMock)

    const result = await loadPublicShareLink("good-token")

    expect(result).toEqual({ status: "ready", file })
    const [url] = fetchMock.mock.calls[0]
    expect(url).toContain("/shared/links/good-token")
  })

  it("maps a 404 to the uniform unavailable message", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      jsonResponse(404, { error: "file_not_found", message: "File was not found" }),
    )
    vi.stubGlobal("fetch", fetchMock)

    const result = await loadPublicShareLink("revoked-token")

    expect(result).toEqual({
      status: "unavailable",
      message:
        "This link is no longer available. It may have been revoked, expired, or the file was deleted.",
    })
  })

  it("does not call the API for an empty token", async () => {
    const fetchMock = vi.fn()
    vi.stubGlobal("fetch", fetchMock)

    const result = await loadPublicShareLink("   ")

    expect(result.status).toBe("unavailable")
    expect(fetchMock).not.toHaveBeenCalled()
  })
})
