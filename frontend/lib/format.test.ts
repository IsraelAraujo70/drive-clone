import { describe, expect, it } from "vitest"
import { formatBytes } from "./format"

describe("formatBytes", () => {
  it("keeps small values in bytes", () => {
    expect(formatBytes(0)).toBe("0 B")
    expect(formatBytes(512)).toBe("512 B")
  })

  it("converts to larger units", () => {
    expect(formatBytes(1024)).toBe("1 KB")
    expect(formatBytes(1536)).toBe("1.5 KB")
    expect(formatBytes(5 * 1024 * 1024)).toBe("5 MB")
  })

  it("formats the 50 MB proof-of-concept quota cleanly", () => {
    expect(formatBytes(52428800)).toBe("50 MB")
  })

  it("never returns negative values", () => {
    expect(formatBytes(-10)).toBe("0 B")
  })
})
