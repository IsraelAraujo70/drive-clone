import { ApiError, UploadPartError } from "./api"

// API error codes that mean the resumable session is gone or no longer usable,
// so the only recovery is to start the upload over.
const EXPIRED_SESSION_CODES = new Set(["invalid_file_state", "file_not_found"])

const API_ERROR_MESSAGES: Record<string, string> = {
  quota_exceeded: "Not enough storage is available for this upload.",
  file_too_large: "This file is larger than the maximum allowed size.",
  unauthorized: "Your session expired. Log in again to keep uploading.",
  validation_error: "The upload request was rejected. Check the file and retry.",
}

export function getErrorMessage(error: unknown): string {
  return error instanceof Error
    ? error.message
    : "Something went wrong. Try again."
}

// Maps any upload failure to a message that tells the user which of the three
// distinct cases happened: a per-part network/storage failure, an expired
// session, or another API error.
export function getUploadErrorMessage(error: unknown): string {
  if (error instanceof UploadPartError) {
    return `Couldn't upload part ${error.partNumber}. Check your connection and resume the upload.`
  }
  if (error instanceof ApiError) {
    if (EXPIRED_SESSION_CODES.has(error.code)) {
      return "This upload expired. Start it again."
    }
    const mapped = API_ERROR_MESSAGES[error.code]
    if (mapped) {
      return mapped
    }
    return `${error.code}: ${error.message}`
  }
  return getErrorMessage(error)
}
