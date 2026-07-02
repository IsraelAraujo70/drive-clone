import type { Metadata } from "next"
import { Bricolage_Grotesque, IBM_Plex_Mono, Inter } from "next/font/google"

import "./globals.css"
import { AuthProvider } from "@/lib/auth"
import { cn } from "@/lib/utils"

const fontSans = Inter({
  subsets: ["latin"],
  variable: "--font-sans",
})

const fontHeading = Bricolage_Grotesque({
  subsets: ["latin"],
  variable: "--font-bricolage",
})

const fontMono = IBM_Plex_Mono({
  subsets: ["latin"],
  weight: ["400", "500"],
  variable: "--font-mono",
})

export const metadata: Metadata = {
  title: "Drive Clone · Personal cloud storage",
  description:
    "Drive Clone is a personal cloud drive with resumable uploads, private-by-default sharing, and 15 GB of free storage.",
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html
      lang="en"
      className={cn(
        "antialiased",
        "font-sans",
        fontSans.variable,
        fontHeading.variable,
        fontMono.variable,
      )}
    >
      <body>
        <AuthProvider>{children}</AuthProvider>
      </body>
    </html>
  )
}
