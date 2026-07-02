import type { Metadata } from "next"

import { Brand } from "@/components/brand"
import { SignupForm } from "@/components/signup-form"

export const metadata: Metadata = {
  title: "Sign up · Drive Clone",
}

export default function SignupPage() {
  return (
    <div className="flex min-h-svh flex-col items-center justify-center gap-8 p-6">
      <Brand />
      <SignupForm />
    </div>
  )
}
