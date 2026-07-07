import type { RefObject } from "react"

export function DriveHiddenFileInputs({
  inputRef,
  resumeInputRef,
  onResumeSelected,
  onUploadSelected,
}: {
  inputRef: RefObject<HTMLInputElement | null>
  resumeInputRef: RefObject<HTMLInputElement | null>
  onResumeSelected: (file: File) => void
  onUploadSelected: (file: File) => void
}) {
  return (
    <>
      <input
        ref={inputRef}
        data-cy="drive-upload-input"
        type="file"
        className="sr-only"
        onChange={(event) => {
          const file = event.target.files?.[0]
          if (file) {
            onUploadSelected(file)
          }
        }}
      />
      <input
        ref={resumeInputRef}
        data-cy="resume-upload-input"
        type="file"
        className="sr-only"
        onChange={(event) => {
          const file = event.target.files?.[0]
          if (file) {
            onResumeSelected(file)
          }
        }}
      />
    </>
  )
}
