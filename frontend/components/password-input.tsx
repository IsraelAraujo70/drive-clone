"use client"

import { CheckIcon, EyeIcon, EyeOffIcon, XIcon } from "lucide-react"
import { useId, useState, type ComponentProps } from "react"

import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupInput,
} from "@/components/ui/input-group"
import { Progress } from "@/components/ui/progress"
import { getPasswordStrength } from "@/lib/passwordStrength"
import { cn } from "@/lib/utils"

type PasswordInputProps = Omit<
  ComponentProps<typeof InputGroupInput>,
  "type"
> & {
  showStrength?: boolean
}

export function PasswordInput({
  showStrength = false,
  value,
  className,
  ...props
}: PasswordInputProps) {
  const [visible, setVisible] = useState(false)
  const generatedId = useId()
  const password = typeof value === "string" ? value : ""
  const strength = getPasswordStrength(password)
  const meterId = `${props.id ?? generatedId}-strength`
  const describedBy = [props["aria-describedby"], showStrength ? meterId : null]
    .filter(Boolean)
    .join(" ")

  return (
    <div className="flex flex-col gap-2">
      <InputGroup className={className}>
        <InputGroupInput
          {...props}
          value={value}
          type={visible ? "text" : "password"}
          aria-describedby={describedBy || undefined}
        />
        <InputGroupAddon align="inline-end">
          <InputGroupButton
            size="icon-xs"
            aria-label={visible ? "Hide password" : "Show password"}
            aria-pressed={visible}
            onClick={() => setVisible((current) => !current)}
          >
            {visible ? <EyeOffIcon /> : <EyeIcon />}
          </InputGroupButton>
        </InputGroupAddon>
      </InputGroup>

      {showStrength && (
        <div id={meterId} className="flex flex-col gap-2">
          <div className="flex items-center justify-between gap-3">
            <Progress value={strength.percent} aria-label="Password strength" />
            <span className="min-w-16 text-right text-xs font-medium text-muted-foreground">
              {strength.label}
            </span>
          </div>
          <ul className="grid gap-1 text-xs text-muted-foreground">
            {strength.checks.map((check) => (
              <li key={check.id} className="flex items-center gap-2">
                <span
                  className={cn(
                    "flex size-4 items-center justify-center rounded-full border",
                    check.met &&
                      "border-primary bg-primary text-primary-foreground"
                  )}
                  aria-hidden="true"
                >
                  {check.met ? <CheckIcon /> : <XIcon />}
                </span>
                <span className={check.met ? "text-foreground" : undefined}>
                  {check.label}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  )
}
