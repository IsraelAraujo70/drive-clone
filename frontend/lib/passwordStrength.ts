export type PasswordStrength = {
  score: number
  label: "Too short" | "Weak" | "Okay" | "Strong"
  percent: number
  checks: {
    id: "length" | "lowerUpper" | "number" | "symbol"
    label: string
    met: boolean
  }[]
}

export function getPasswordStrength(password: string): PasswordStrength {
  const checks = [
    {
      id: "length" as const,
      label: "At least 8 characters",
      met: password.length >= 8,
    },
    {
      id: "lowerUpper" as const,
      label: "Upper and lowercase letters",
      met: /[a-z]/.test(password) && /[A-Z]/.test(password),
    },
    {
      id: "number" as const,
      label: "At least one number",
      met: /\d/.test(password),
    },
    {
      id: "symbol" as const,
      label: "At least one symbol",
      met: /[^A-Za-z0-9]/.test(password),
    },
  ]
  const score = checks.filter((check) => check.met).length
  const label =
    password.length > 0 && password.length < 8
      ? "Too short"
      : score >= 4
        ? "Strong"
        : score >= 3
          ? "Okay"
          : "Weak"

  return {
    score,
    label,
    percent: password.length === 0 ? 0 : Math.max(10, score * 25),
    checks,
  }
}

export function isStrongPassword(password: string) {
  return getPasswordStrength(password).score >= 4
}
