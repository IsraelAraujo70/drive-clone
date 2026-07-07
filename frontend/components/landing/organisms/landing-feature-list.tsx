import { LandingFeatureRow } from "@/components/landing/molecules/landing-feature-row"

const listing = [
  {
    path: "/uploads",
    title: "Signed direct uploads",
    copy: "The API checks quota, signs a short-lived upload URL, then verifies the stored object before it appears.",
  },
  {
    path: "/folders",
    title: "Organize without fear",
    copy: "Rename, move, trash and restore. Deleted files wait in trash until you decide.",
  },
  {
    path: "/shared",
    title: "Private by default",
    copy: "Grant access to a registered user's email, revoke that access, and keep every other file private.",
  },
  {
    path: "/search",
    title: "Found by name, instantly",
    copy: "Search everything you own. Results never include files you cannot access.",
  },
]

export function LandingFeatureList() {
  return (
    <section
      aria-label="What your drive can do"
      className="mx-auto w-full max-w-6xl px-6 pb-24"
    >
      <div className="flex items-center justify-between border-b border-border pb-3 font-mono text-xs tracking-wider text-muted-foreground uppercase">
        <span>~/drive</span>
        <span>{listing.length} entries</span>
      </div>
      {listing.map((entry) => (
        <LandingFeatureRow key={entry.path} {...entry} />
      ))}
    </section>
  )
}
