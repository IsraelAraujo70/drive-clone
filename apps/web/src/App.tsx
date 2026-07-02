import { useEffect, useState } from "react";
import "./App.css";
import { formatHealthStatus, type ApiHealth } from "./health";

const apiBaseUrl =
  import.meta.env.VITE_API_BASE_URL ?? "https://api-production-bcad4.up.railway.app";

type ApiState =
  | { status: "loading"; label: string }
  | { status: "ok"; label: string }
  | { status: "error"; label: string };

const files = [
  { name: "Architecture notes.md", type: "Markdown", updated: "Today" },
  { name: "Product screenshots.zip", type: "Archive", updated: "Yesterday" },
  { name: "Resume.pdf", type: "PDF", updated: "Jun 29" },
];

function App() {
  const [apiState, setApiState] = useState<ApiState>({
    status: "loading",
    label: "Checking API",
  });

  useEffect(() => {
    const controller = new AbortController();

    fetch(`${apiBaseUrl}/health`, { signal: controller.signal })
      .then((response) => {
        if (!response.ok) {
          throw new Error(`API returned ${response.status}`);
        }

        return response.json() as Promise<ApiHealth>;
      })
      .then((health) => {
        setApiState({ status: "ok", label: formatHealthStatus(health) });
      })
      .catch((error: Error) => {
        if (error.name !== "AbortError") {
          setApiState({ status: "error", label: "API offline" });
        }
      });

    return () => controller.abort();
  }, []);

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
          <strong>2.1 GB of 15 GB</strong>
          <div className="storage-bar" aria-label="Storage usage">
            <div className="storage-fill" />
          </div>
          <span className="storage-copy">Storage usage preview</span>
        </div>
      </aside>

      <main className="main">
        <header className="topbar">
          <div className="search">Search files</div>
          <div className={`status-pill ${apiState.status}`}>{apiState.label}</div>
        </header>

        <section className="content">
          <div className="heading-row">
            <div>
              <h1>My Drive</h1>
              <p className="muted">Initial frontend scaffold connected to the API health endpoint.</p>
            </div>
          </div>

          <div className="grid">
            <div className="tile">
              <span className="tile-label">Files</span>
              <span className="tile-value">3</span>
            </div>
            <div className="tile">
              <span className="tile-label">Folders</span>
              <span className="tile-value">1</span>
            </div>
            <div className="tile">
              <span className="tile-label">Upload status</span>
              <span className="tile-value">Ready</span>
            </div>
          </div>

          <div className="table-panel">
            <div className="table-header">
              <span>Name</span>
              <span>Type</span>
              <span>Updated</span>
            </div>
            {files.map((file) => (
              <div className="file-row" key={file.name}>
                <span className="file-name">{file.name}</span>
                <span className="muted">{file.type}</span>
                <span className="muted">{file.updated}</span>
              </div>
            ))}
          </div>
        </section>
      </main>
    </div>
  );
}

export default App;
