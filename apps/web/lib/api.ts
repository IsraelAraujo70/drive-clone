export const API_BASE_URL =
  process.env.NEXT_PUBLIC_API_BASE_URL ??
  (process.env.NODE_ENV === "development"
    ? "http://localhost:8080"
    : "https://api-production-bcad4.up.railway.app")

export type User = {
  id: string
  email: string
  display_name: string
  storage_quota_bytes: number
  storage_used_bytes: number
  created_at: string
}

export type AuthResponse = {
  user: User
  token: string
}

export class ApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
  ) {
    super(message)
    this.name = "ApiError"
  }
}

type RequestOptions = {
  method?: string
  token?: string | null
  body?: unknown
}

async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const headers: Record<string, string> = {}
  if (options.body !== undefined) {
    headers["Content-Type"] = "application/json"
  }
  if (options.token) {
    headers.Authorization = `Bearer ${options.token}`
  }

  const response = await fetch(`${API_BASE_URL}${path}`, {
    method: options.method ?? "GET",
    headers,
    body: options.body !== undefined ? JSON.stringify(options.body) : undefined,
  })

  if (response.status === 204) {
    return undefined as T
  }

  const data = await response.json().catch(() => null)
  if (!response.ok) {
    throw new ApiError(
      response.status,
      data?.error ?? "unknown_error",
      data?.message ?? "Something went wrong. Try again.",
    )
  }
  return data as T
}

export const api = {
  signup: (input: { email: string; password: string; display_name: string }) =>
    request<AuthResponse>("/auth/signup", { method: "POST", body: input }),
  login: (input: { email: string; password: string }) =>
    request<AuthResponse>("/auth/login", { method: "POST", body: input }),
  logout: (token: string) => request<void>("/auth/logout", { method: "POST", token }),
  me: (token: string) => request<User>("/auth/me", { token }),
  health: () => request<{ status: string; service: string }>("/health"),
}
