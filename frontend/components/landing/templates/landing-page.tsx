import { LandingFeatureList } from "@/components/landing/organisms/landing-feature-list"
import { LandingFooter } from "@/components/landing/organisms/landing-footer"
import { LandingHeader } from "@/components/landing/organisms/landing-header"
import { LandingHero } from "@/components/landing/organisms/landing-hero"
import { QuotaSection } from "@/components/landing/organisms/quota-section"

export function LandingPage() {
  return (
    <div className="flex min-h-svh flex-col">
      <LandingHeader />
      <main className="flex-1">
        <LandingHero />
        <LandingFeatureList />
        <QuotaSection />
      </main>
      <LandingFooter />
    </div>
  )
}
