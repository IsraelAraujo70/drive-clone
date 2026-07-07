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
import { ApiError, api } from "@/lib/api"
import { useAuth } from "@/lib/auth"

export function LoginForm() {
  const { login } = useAuth()
  const router = useRouter()
  const [email, setEmail] = useState("")
  const [password, setPassword] = useState("")
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [pending, setPending] = useState(false)
  const [forgotMode, setForgotMode] = useState(false)

  async function handleSubmit(event: FormEvent) {
    event.preventDefault()
    setError(null)
    setNotice(null)
    setPending(true)
    try {
      if (forgotMode) {
        await api.requestPasswordReset({ email })
        setNotice("If that email has an account, a reset link is on the way.")
        setPending(false)
        return
      }

      await login({ email, password })
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
          <CardTitle className="font-heading text-2xl">Welcome back</CardTitle>
          <CardDescription>
            {forgotMode
              ? "Send a password reset link to your email."
              : "Log in to access your drive."}
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-6">
          {error && (
            <Alert variant="destructive" role="alert">
              <AlertTitle>{error}</AlertTitle>
            </Alert>
          )}
          {notice && (
            <Alert role="status">
              <AlertTitle>{notice}</AlertTitle>
            </Alert>
          )}
          <FieldGroup>
            <Field>
              <FieldLabel htmlFor="email">Email</FieldLabel>
              <Input
                id="email"
                data-cy="login-email"
                type="email"
                required
                autoComplete="email"
                value={email}
                onChange={(event) => setEmail(event.target.value)}
              />
            </Field>
            {!forgotMode && (
              <Field>
                <div className="flex items-center justify-between gap-3">
                  <FieldLabel htmlFor="password">Password</FieldLabel>
                  <Button
                    type="button"
                    variant="link"
                    size="xs"
                    className="h-auto px-0"
                    onClick={() => {
                      setForgotMode(true)
                      setError(null)
                      setNotice(null)
                    }}
                  >
                    Forgot password?
                  </Button>
                </div>
                <PasswordInput
                  id="password"
                  data-cy="login-password"
                  required
                  autoComplete="current-password"
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                />
              </Field>
            )}
            {forgotMode && (
              <Field>
                <FieldDescription>
                  The reset link expires in one hour.
                </FieldDescription>
              </Field>
            )}
          </FieldGroup>
        </CardContent>
        <CardFooter className="flex flex-col gap-4">
          <Button
            type="submit"
            data-cy="login-submit"
            className="w-full"
            disabled={pending}
          >
            {pending && <Spinner data-icon="inline-start" />}
            {pending
              ? forgotMode
                ? "Sending link…"
                : "Logging in…"
              : forgotMode
                ? "Send reset link"
                : "Log in"}
          </Button>
          {forgotMode && (
            <Button
              type="button"
              variant="ghost"
              className="w-full"
              onClick={() => {
                setForgotMode(false)
                setError(null)
                setNotice(null)
              }}
            >
              Back to log in
            </Button>
          )}
          <p className="text-sm text-muted-foreground">
            New here?{" "}
            <Link href="/signup" className="text-primary hover:underline">
              Create an account
            </Link>
          </p>
        </CardFooter>
      </Card>
    </form>
  )
}
