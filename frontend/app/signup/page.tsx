import type { Metadata } from "next"

import { SignupForm } from "@/components/auth/organisms/signup-form"
import { AuthPageShell } from "@/components/auth/templates/auth-page-shell"

export const metadata: Metadata = {
  title: "Sign up · Drive Clone",
}

export default function SignupPage() {
  return (
    <AuthPageShell>
      <SignupForm />
    </AuthPageShell>
  )
}
