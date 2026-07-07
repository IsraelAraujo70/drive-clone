import type { Metadata } from "next"

import { Brand } from "@/components/brand"
import { LoginForm } from "@/components/login-form"

export const metadata: Metadata = {
  title: "Log in · Drive Clone",
}

export default function LoginPage() {
  return (
    <div className="flex min-h-svh flex-col items-center justify-center gap-8 p-6">
      <Brand />
      <LoginForm />
    </div>
  )
}
