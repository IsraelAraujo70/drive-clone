import type { NextConfig } from "next"

const nextConfig: NextConfig = {
  allowedDevOrigins: ["web-e2e"],
  output: "standalone",
}

export default nextConfig
