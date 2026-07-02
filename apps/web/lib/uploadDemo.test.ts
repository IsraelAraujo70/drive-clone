import { describe, expect, it } from "vitest"
import {
  INTERRUPT_AT,
  TOTAL_CHUNKS,
  buildUploadFrames,
  statusFor,
} from "./uploadDemo"

describe("upload demo timeline", () => {
  const frames = buildUploadFrames()

  it("starts uploading from the first chunk", () => {
    expect(frames[0]).toEqual({ phase: "uploading", progress: 1 })
  })

  it("never loses confirmed progress", () => {
    for (let i = 1; i < frames.length; i += 1) {
      expect(frames[i].progress).toBeGreaterThanOrEqual(frames[i - 1].progress)
    }
  })

  it("holds at the interruption point while disconnected", () => {
    const interrupted = frames.filter((frame) => frame.phase === "interrupted")
    expect(interrupted.length).toBeGreaterThan(0)
    for (const frame of interrupted) {
      expect(frame.progress).toBe(INTERRUPT_AT)
    }
  })

  it("resumes after the break instead of restarting", () => {
    const firstResumed = frames.find((frame) => frame.phase === "resumed")
    expect(firstResumed?.progress).toBe(INTERRUPT_AT + 1)
  })

  it("ends complete with every chunk confirmed", () => {
    const last = frames[frames.length - 1]
    expect(last.phase).toBe("complete")
    expect(last.progress).toBe(TOTAL_CHUNKS)
  })

  it("describes each phase for the status line", () => {
    expect(statusFor({ phase: "uploading", progress: 3 })).toBe(
      "Uploading · chunk 3 of 30",
    )
    expect(statusFor({ phase: "interrupted", progress: 12 })).toBe(
      "Connection lost · progress saved at chunk 12",
    )
    expect(statusFor({ phase: "resumed", progress: 20 })).toBe(
      "Resumed · chunk 20 of 30",
    )
    expect(statusFor({ phase: "complete", progress: 30 })).toBe(
      "Complete · checksum verified",
    )
  })
})
