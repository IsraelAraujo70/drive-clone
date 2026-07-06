import { describe, expect, it } from "vitest"

import { TRASH_RETENTION_DAYS, trashPolicyCopy } from "./trashPolicy"

describe("trash policy copy", () => {
  it("communicates the permanent purge window", () => {
    expect(TRASH_RETENTION_DAYS).toBe(30)
    expect(trashPolicyCopy.description).toContain("30 days")
    expect(trashPolicyCopy.description).toContain("permanently deleted")
    expect(trashPolicyCopy.emptyDescription).toContain("30 days")
  })
})
