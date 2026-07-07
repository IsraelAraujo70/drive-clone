import type { Metadata } from "next"

import { ResetPasswordForm } from "@/components/auth/organisms/reset-password-form"
import { AuthPageShell } from "@/components/auth/templates/auth-page-shell"

export const metadata: Metadata = {
  title: "Reset password · Drive Clone",
}

export default async function ResetPasswordPage({
  searchParams,
}: {
  searchParams: Promise<{ token?: string }>
}) {
  const { token = "" } = await searchParams

  return (
    <AuthPageShell>
      <ResetPasswordForm token={token} />
    </AuthPageShell>
  )
}
