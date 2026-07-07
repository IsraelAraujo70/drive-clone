import type { Metadata } from "next"

import { LoginForm } from "@/components/auth/organisms/login-form"
import { AuthPageShell } from "@/components/auth/templates/auth-page-shell"

export const metadata: Metadata = {
  title: "Log in · Drive Clone",
}

export default function LoginPage() {
  return (
    <AuthPageShell>
      <LoginForm />
    </AuthPageShell>
  )
}
