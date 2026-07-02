import {
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { Navigate } from "react-router-dom";
import { api, type AuthResponse, type User } from "./api";

const TOKEN_KEY = "drive_clone_token";

type AuthContextValue = {
  user: User | null;
  loading: boolean;
  signup: (input: { email: string; password: string; display_name: string }) => Promise<void>;
  login: (input: { email: string; password: string }) => Promise<void>;
  logout: () => Promise<void>;
};

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(() => localStorage.getItem(TOKEN_KEY) !== null);
  const tokenRef = useRef<string | null>(localStorage.getItem(TOKEN_KEY));

  useEffect(() => {
    const stored = localStorage.getItem(TOKEN_KEY);
    if (!stored) {
      return;
    }
    let cancelled = false;
    api
      .me(stored)
      .then((me) => {
        if (!cancelled) {
          setUser(me);
        }
      })
      .catch(() => {
        if (!cancelled) {
          localStorage.removeItem(TOKEN_KEY);
          tokenRef.current = null;
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const applyAuth = (response: AuthResponse) => {
    localStorage.setItem(TOKEN_KEY, response.token);
    tokenRef.current = response.token;
    setUser(response.user);
    setLoading(false);
  };

  const value: AuthContextValue = {
    user,
    loading,
    signup: async (input) => applyAuth(await api.signup(input)),
    login: async (input) => applyAuth(await api.login(input)),
    logout: async () => {
      const token = tokenRef.current;
      localStorage.removeItem(TOKEN_KEY);
      tokenRef.current = null;
      setUser(null);
      if (token) {
        await api.logout(token).catch(() => undefined);
      }
    },
  };

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error("useAuth must be used inside AuthProvider");
  }
  return context;
}

export function RequireAuth({ children }: { children: ReactNode }) {
  const { user, loading } = useAuth();
  if (loading) {
    return <div className="page-loading">Loading your drive…</div>;
  }
  if (!user) {
    return <Navigate to="/login" replace />;
  }
  return <>{children}</>;
}
