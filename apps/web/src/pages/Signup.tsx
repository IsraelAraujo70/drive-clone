import { useState, type FormEvent } from "react";
import { Link, useNavigate } from "react-router-dom";
import { ApiError } from "../lib/api";
import { useAuth } from "../lib/auth";

export default function Signup() {
  const { signup } = useAuth();
  const navigate = useNavigate();
  const [displayName, setDisplayName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    setError(null);
    setPending(true);
    try {
      await signup({ email, password, display_name: displayName });
      navigate("/drive", { replace: true });
    } catch (caught) {
      setError(
        caught instanceof ApiError
          ? caught.message
          : "Could not reach the server. Try again.",
      );
    } finally {
      setPending(false);
    }
  }

  return (
    <div className="auth-page">
      <Link className="brand" to="/">
        <div className="brand-mark">D</div>
        <span>Drive Clone</span>
      </Link>

      <form className="auth-card" onSubmit={handleSubmit}>
        <h1>Create your account</h1>
        <p className="muted">15 GB of free storage, private by default.</p>

        {error && (
          <div className="form-error" role="alert">
            {error}
          </div>
        )}

        <label>
          Name
          <input
            type="text"
            required
            maxLength={100}
            autoComplete="name"
            value={displayName}
            onChange={(event) => setDisplayName(event.target.value)}
          />
        </label>

        <label>
          Email
          <input
            type="email"
            required
            autoComplete="email"
            value={email}
            onChange={(event) => setEmail(event.target.value)}
          />
        </label>

        <label>
          Password
          <input
            type="password"
            required
            minLength={8}
            maxLength={128}
            autoComplete="new-password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
          />
          <span className="field-hint">At least 8 characters.</span>
        </label>

        <button className="button primary large" type="submit" disabled={pending}>
          {pending ? "Creating account…" : "Sign up"}
        </button>

        <p className="muted">
          Already have an account? <Link to="/login">Log in</Link>
        </p>
      </form>
    </div>
  );
}
