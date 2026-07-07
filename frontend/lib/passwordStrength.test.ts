import { describe, expect, it } from "vitest"

import { getPasswordStrength, isStrongPassword } from "./passwordStrength"

describe("password strength", () => {
  it("requires length, mixed case, number, and symbol for a strong password", () => {
    expect(isStrongPassword("password123")).toBe(false)
    expect(isStrongPassword("Password123")).toBe(false)
    expect(isStrongPassword("Password123!")).toBe(true)
  })

  it("returns checklist state for the UI", () => {
    const strength = getPasswordStrength("Pass123!")

    expect(strength.label).toBe("Strong")
    expect(strength.percent).toBe(100)
    expect(strength.checks).toEqual([
      { id: "length", label: "At least 8 characters", met: true },
      { id: "lowerUpper", label: "Upper and lowercase letters", met: true },
      { id: "number", label: "At least one number", met: true },
      { id: "symbol", label: "At least one symbol", met: true },
    ])
  })
})
