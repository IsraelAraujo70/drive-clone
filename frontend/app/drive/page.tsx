import type { Metadata } from "next"

import { DriveShell } from "@/components/drive/templates/drive-shell"
import { RequireAuth } from "@/lib/auth"

export const metadata: Metadata = {
  title: "My Drive · Drive Clone",
}

export default function DrivePage() {
  return (
    <RequireAuth>
      <DriveShell />
    </RequireAuth>
  )
}
