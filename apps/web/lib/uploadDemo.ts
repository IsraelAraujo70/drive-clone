export type UploadPhase = "uploading" | "interrupted" | "resumed" | "complete"

export type UploadFrame = {
  phase: UploadPhase
  progress: number
}

export const TOTAL_CHUNKS = 30
export const INTERRUPT_AT = 12

const HOLD_INTERRUPTED = 11
const HOLD_COMPLETE = 22

export function buildUploadFrames(): UploadFrame[] {
  return [
    ...Array.from({ length: INTERRUPT_AT }, (_, i) => ({
      phase: "uploading" as const,
      progress: i + 1,
    })),
    ...Array.from({ length: HOLD_INTERRUPTED }, () => ({
      phase: "interrupted" as const,
      progress: INTERRUPT_AT,
    })),
    ...Array.from({ length: TOTAL_CHUNKS - INTERRUPT_AT }, (_, i) => ({
      phase: "resumed" as const,
      progress: INTERRUPT_AT + i + 1,
    })),
    ...Array.from({ length: HOLD_COMPLETE }, () => ({
      phase: "complete" as const,
      progress: TOTAL_CHUNKS,
    })),
  ]
}

export const uploadFrames = buildUploadFrames()

export function statusFor(frame: UploadFrame): string {
  switch (frame.phase) {
    case "uploading":
      return `Uploading · chunk ${frame.progress} of ${TOTAL_CHUNKS}`
    case "interrupted":
      return `Connection lost · progress saved at chunk ${INTERRUPT_AT}`
    case "resumed":
      return `Resumed · chunk ${frame.progress} of ${TOTAL_CHUNKS}`
    case "complete":
      return "Complete · checksum verified"
  }
}
