import { PublicSharePage } from "@/components/share/templates/public-share-page"

export default async function SharedFilePage({
  params,
}: {
  params: Promise<{ token: string }>
}) {
  const { token } = await params

  return <PublicSharePage token={token} />
}
