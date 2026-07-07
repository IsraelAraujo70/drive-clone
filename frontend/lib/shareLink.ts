import { api, type PublicShareLink } from "./api"
import { getShareLinkErrorMessage } from "./shareErrors"

export type PublicShareLinkState =
  | { status: "ready"; file: PublicShareLink }
  | { status: "unavailable"; message: string }

/**
 * Resolve a public share link into either the file metadata or a friendly
 * message. Every failure (bad token, revoked, expired, trashed file) maps to a
 * single unavailable state so the page never leaks why a link failed.
 */
export async function loadPublicShareLink(
  token: string
): Promise<PublicShareLinkState> {
  const trimmed = token.trim()
  if (!trimmed) {
    return {
      status: "unavailable",
      message: getShareLinkErrorMessage(new Error("Missing share link token.")),
    }
  }

  try {
    const file = await api.resolveShareLink(trimmed)
    return { status: "ready", file }
  } catch (caught) {
    return { status: "unavailable", message: getShareLinkErrorMessage(caught) }
  }
}
