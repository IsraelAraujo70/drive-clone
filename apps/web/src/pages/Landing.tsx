import { Link } from "react-router-dom";
import { useAuth } from "../lib/auth";

const features = [
  {
    title: "Upload anything",
    copy: "Drop files into your drive and keep them safe in the cloud, with 15 GB of free storage.",
  },
  {
    title: "Organize with folders",
    copy: "Folders, renames, moves, trash and restore. Your files, your structure.",
  },
  {
    title: "Share securely",
    copy: "Private by default. Share files through revocable links only when you choose to.",
  },
  {
    title: "Find it fast",
    copy: "Search your entire drive by name and get results only you can see.",
  },
];

export default function Landing() {
  const { user } = useAuth();

  return (
    <div className="landing">
      <header className="landing-nav">
        <div className="brand">
          <div className="brand-mark">D</div>
          <span>Drive Clone</span>
        </div>
        <nav className="landing-actions">
          {user ? (
            <Link className="button primary" to="/drive">
              Open your drive
            </Link>
          ) : (
            <>
              <Link className="button ghost" to="/login">
                Log in
              </Link>
              <Link className="button primary" to="/signup">
                Sign up
              </Link>
            </>
          )}
        </nav>
      </header>

      <main>
        <section className="hero">
          <h1>Your files, everywhere you are</h1>
          <p>
            Store, organize and share your files from any browser. 15 GB of free
            storage, private by default, built to never lose an upload.
          </p>
          <div className="hero-actions">
            <Link className="button primary large" to={user ? "/drive" : "/signup"}>
              {user ? "Open your drive" : "Get started for free"}
            </Link>
            {!user && (
              <Link className="button ghost large" to="/login">
                I already have an account
              </Link>
            )}
          </div>
        </section>

        <section className="features">
          {features.map((feature) => (
            <article className="feature-card" key={feature.title}>
              <h2>{feature.title}</h2>
              <p>{feature.copy}</p>
            </article>
          ))}
        </section>
      </main>

      <footer className="landing-footer">
        <span>Drive Clone — a portfolio cloud storage project built with Rust and React.</span>
      </footer>
    </div>
  );
}
