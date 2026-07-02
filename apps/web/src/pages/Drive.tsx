import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { formatHealthStatus } from "../health";
import { api } from "../lib/api";
import { useAuth } from "../lib/auth";
import { formatBytes } from "../lib/format";

type ApiState =
  | { status: "loading"; label: string }
  | { status: "ok"; label: string }
  | { status: "error"; label: string };

export default function Drive() {
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  const [apiState, setApiState] = useState<ApiState>({
    status: "loading",
    label: "Checking API",
  });

  useEffect(() => {
    let cancelled = false;
    api
      .health()
      .then((health) => {
        if (!cancelled) {
          setApiState({ status: "ok", label: formatHealthStatus(health) });
        }
      })
      .catch(() => {
        if (!cancelled) {
          setApiState({ status: "error", label: "API offline" });
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (!user) {
    return null; // RequireAuth guarantees a user; this satisfies the type checker
  }

  const usedPercent = Math.min(
    100,
    (user.storage_used_bytes / user.storage_quota_bytes) * 100,
  );

  async function handleLogout() {
    await logout();
    navigate("/", { replace: true });
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">D</div>
          <span>Drive Clone</span>
        </div>

        <nav className="nav" aria-label="Drive sections">
          <button className="nav-item active">My Drive</button>
          <button className="nav-item">Shared</button>
          <button className="nav-item">Trash</button>
        </nav>

        <div className="storage-panel">
          <strong>
            {formatBytes(user.storage_used_bytes)} of {formatBytes(user.storage_quota_bytes)}
          </strong>
          <div className="storage-bar" aria-label="Storage usage">
            <div className="storage-fill" style={{ width: `${usedPercent}%` }} />
          </div>
          <span className="storage-copy">Storage used</span>
        </div>
      </aside>

      <main className="main">
        <header className="topbar">
          <div className="search">Search files</div>
          <div className="topbar-actions">
            <div className={`status-pill ${apiState.status}`}>{apiState.label}</div>
            <div className="user-chip" title={user.email}>
              {user.display_name}
            </div>
            <button className="button ghost" type="button" onClick={handleLogout}>
              Log out
            </button>
          </div>
        </header>

        <section className="content">
          <div className="heading-row">
            <div>
              <h1>My Drive</h1>
              <p className="muted">Signed in as {user.email}.</p>
            </div>
          </div>

          <div className="grid">
            <div className="tile">
              <span className="tile-label">Files</span>
              <span className="tile-value">0</span>
            </div>
            <div className="tile">
              <span className="tile-label">Folders</span>
              <span className="tile-value">0</span>
            </div>
            <div className="tile">
              <span className="tile-label">Storage used</span>
              <span className="tile-value">{formatBytes(user.storage_used_bytes)}</span>
            </div>
          </div>

          <div className="table-panel">
            <div className="table-header">
              <span>Name</span>
              <span>Type</span>
              <span>Updated</span>
            </div>
            <div className="empty-state">
              No files yet. Uploads arrive in the next milestone.
            </div>
          </div>
        </section>
      </main>
    </div>
  );
}
