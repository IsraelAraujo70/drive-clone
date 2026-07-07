import type { Metadata } from "next"

import { Brand } from "@/components/brand"
import { ResetPasswordForm } from "@/components/reset-password-form"

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
    <div className="flex min-h-svh flex-col items-center justify-center gap-8 p-6">
      <Brand />
      <ResetPasswordForm token={token} />
    </div>
  )
}
