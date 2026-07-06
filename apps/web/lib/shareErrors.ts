import { ApiError } from "./api"

export function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "Something went wrong. Try again."
}

export function getApiErrorMessage(error: unknown): string {
  if (error instanceof ApiError) {
    return `${error.code}: ${error.message}`
  }
  return getErrorMessage(error)
}

export function getShareErrorMessage(error: unknown, email: string): string {
  if (error instanceof ApiError && error.code === "user_not_found") {
    return `No account uses ${email}. Ask them to sign up first, then share this file again.`
  }
  return getApiErrorMessage(error)
}
