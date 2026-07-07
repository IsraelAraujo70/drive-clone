"use client"

import {
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react"
import { useRouter } from "next/navigation"

import { Spinner } from "@/components/ui/spinner"
import { api, type AuthResponse, type User } from "@/lib/api"

const TOKEN_KEY = "drive_clone_token"

type AuthContextValue = {
  user: User | null
  token: string | null
  loading: boolean
  refreshUser: () => Promise<User | null>
  signup: (input: {
    email: string
    password: string
    display_name: string
  }) => Promise<void>
  login: (input: { email: string; password: string }) => Promise<void>
  logout: () => Promise<void>
}

const AuthContext = createContext<AuthContextValue | null>(null)

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null)
  const [token, setToken] = useState<string | null>(null)
  // Starts true and settles on the client; the stored token is only readable there.
  const [loading, setLoading] = useState(true)
  const tokenRef = useRef<string | null>(null)

  useEffect(() => {
    let cancelled = false
    const stored = localStorage.getItem(TOKEN_KEY)
    tokenRef.current = stored

    const settle = stored
      ? api.me(stored).then(
          (me) => {
            if (!cancelled) {
              setToken(stored)
              setUser(me)
            }
          },
          () => {
            if (!cancelled) {
              localStorage.removeItem(TOKEN_KEY)
              tokenRef.current = null
            }
          },
        )
      : Promise.resolve()

    settle.finally(() => {
      if (!cancelled) {
        setLoading(false)
      }
    })

    return () => {
      cancelled = true
    }
  }, [])

  const applyAuth = (response: AuthResponse) => {
    localStorage.setItem(TOKEN_KEY, response.token)
    tokenRef.current = response.token
    setToken(response.token)
    setUser(response.user)
    setLoading(false)
  }

  const refreshUser = async () => {
    const currentToken = tokenRef.current
    if (!currentToken) {
      return null
    }

    const me = await api.me(currentToken)
    setUser(me)
    return me
  }

  const value: AuthContextValue = {
    user,
    token,
    loading,
    refreshUser,
    signup: async (input) => applyAuth(await api.signup(input)),
    login: async (input) => applyAuth(await api.login(input)),
    logout: async () => {
      const token = tokenRef.current
      localStorage.removeItem(TOKEN_KEY)
      tokenRef.current = null
      setToken(null)
      setUser(null)
      if (token) {
        await api.logout(token).catch(() => undefined)
      }
    },
  }

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}

export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext)
  if (!context) {
    throw new Error("useAuth must be used inside AuthProvider")
  }
  return context
}

export function RequireAuth({ children }: { children: ReactNode }) {
  const { user, loading } = useAuth()
  const router = useRouter()

  useEffect(() => {
    if (!loading && !user) {
      router.replace("/login")
    }
  }, [loading, user, router])

  if (loading || !user) {
    return (
      <div className="grid min-h-svh place-items-center">
        <div className="text-muted-foreground flex items-center gap-2 text-sm">
          <Spinner />
          Loading your drive…
        </div>
      </div>
    )
  }

  return <>{children}</>
}
