"use client"

import { useState, type FormEvent } from "react"
import Link from "next/link"
import { useRouter } from "next/navigation"

import { Alert, AlertTitle } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { PasswordInput } from "@/components/password-input"
import { Spinner } from "@/components/ui/spinner"
import { ApiError } from "@/lib/api"
import { useAuth } from "@/lib/auth"
import { isStrongPassword } from "@/lib/passwordStrength"

export function SignupForm() {
  const { signup } = useAuth()
  const router = useRouter()
  const [displayName, setDisplayName] = useState("")
  const [email, setEmail] = useState("")
  const [password, setPassword] = useState("")
  const [confirmPassword, setConfirmPassword] = useState("")
  const [error, setError] = useState<string | null>(null)
  const [pending, setPending] = useState(false)
  const passwordsMismatch =
    confirmPassword.length > 0 && password !== confirmPassword
  const passwordReady = isStrongPassword(password)

  async function handleSubmit(event: FormEvent) {
    event.preventDefault()
    setError(null)
    if (!passwordReady) {
      setError("Use a stronger password before creating your account.")
      return
    }
    if (password !== confirmPassword) {
      setError("Passwords do not match.")
      return
    }
    setPending(true)
    try {
      await signup({ email, password, display_name: displayName })
      router.replace("/drive")
    } catch (caught) {
      setError(
        caught instanceof ApiError
          ? caught.message
          : "Could not reach the server. Try again."
      )
      setPending(false)
    }
  }

  return (
    <form onSubmit={handleSubmit} className="w-full max-w-sm">
      <Card>
        <CardHeader>
          <CardTitle className="font-heading text-2xl">
            Create your account
          </CardTitle>
          <CardDescription>
            50 MB of free storage, private by default.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-6">
          {error && (
            <Alert variant="destructive" role="alert">
              <AlertTitle>{error}</AlertTitle>
            </Alert>
          )}
          <FieldGroup>
            <Field>
              <FieldLabel htmlFor="name">Name</FieldLabel>
              <Input
                id="name"
                type="text"
                required
                maxLength={100}
                autoComplete="name"
                value={displayName}
                onChange={(event) => setDisplayName(event.target.value)}
              />
            </Field>
            <Field>
              <FieldLabel htmlFor="email">Email</FieldLabel>
              <Input
                id="email"
                type="email"
                required
                autoComplete="email"
                value={email}
                onChange={(event) => setEmail(event.target.value)}
              />
            </Field>
            <Field data-invalid={!passwordReady && password.length > 0}>
              <FieldLabel htmlFor="password">Password</FieldLabel>
              <PasswordInput
                id="password"
                required
                minLength={8}
                maxLength={128}
                autoComplete="new-password"
                value={password}
                showStrength
                aria-invalid={!passwordReady && password.length > 0}
                onChange={(event) => setPassword(event.target.value)}
              />
            </Field>
            <Field data-invalid={passwordsMismatch}>
              <FieldLabel htmlFor="confirm-password">
                Confirm password
              </FieldLabel>
              <PasswordInput
                id="confirm-password"
                required
                minLength={8}
                maxLength={128}
                autoComplete="new-password"
                value={confirmPassword}
                aria-invalid={passwordsMismatch}
                onChange={(event) => setConfirmPassword(event.target.value)}
              />
              {passwordsMismatch && (
                <FieldDescription>Passwords do not match.</FieldDescription>
              )}
            </Field>
          </FieldGroup>
        </CardContent>
        <CardFooter className="flex flex-col gap-4">
          <Button
            type="submit"
            className="w-full"
            disabled={pending || !passwordReady || password !== confirmPassword}
          >
            {pending && <Spinner data-icon="inline-start" />}
            {pending ? "Creating account…" : "Sign up"}
          </Button>
          <p className="text-sm text-muted-foreground">
            Already have an account?{" "}
            <Link href="/login" className="text-primary hover:underline">
              Log in
            </Link>
          </p>
        </CardFooter>
      </Card>
    </form>
  )
}
