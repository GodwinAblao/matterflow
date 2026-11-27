import { AboutProject } from "./features/about/AboutProject";
import { ApiStatus } from "./features/health/ApiStatus";
import { DemoEntry } from "./features/auth/DemoEntry";
import { useDemoSession } from "./features/auth/useDemoSession";
import { SessionWorkspace } from "./features/matters/DemoWorkspace";
import type { Role } from "./features/auth/useDemoSession";
import "./App.css";

export default function App() {
  const demo = useDemoSession();
  return (
    <main className={"shell" + (demo.session ? " has-session" : "")}>
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            M
          </span>{" "}
          MatterFlow
        </div>
        <div className="topbar-actions">
          <AboutProject />
          {demo.session && (
            <div className="session-controls">
              <label>
                <span className="sr-only">Demo role</span>
                <select
                  value={demo.session.role}
                  disabled={demo.busy}
                  onChange={(event) =>
                    void demo.switchRole(event.target.value as Role)
                  }
                >
                  <option value="staff">Paralegal / Staff</option>
                  <option value="reviewer">Lawyer / Reviewer</option>
                </select>
              </label>
              <button disabled={demo.busy} onClick={() => void demo.end()}>
                End demo
              </button>
            </div>
          )}
        </div>
      </header>
      {demo.error && (
        <p className="notice error" role="alert">
          {demo.error}
        </p>
      )}
      {demo.restoring ? (
        <p className="loading" role="status">
          Restoring your demo session...
        </p>
      ) : demo.session && demo.token ? (
        <>
          <div className="session-strip">
            <span>Welcome, {demo.session.display_name}</span>
            <span>
              Demo expires at{" "}
              {new Intl.DateTimeFormat("en-PH", {
                hour: "numeric",
                minute: "2-digit",
                timeZone: "Asia/Manila",
              }).format(new Date(demo.session.expires_at))}{" "}
              PH time
            </span>
          </div>
          <div aria-busy={demo.busy}>
            <SessionWorkspace
              key={demo.session.workspace_id}
              session={demo.session}
              token={demo.token}
              onExpired={demo.expire}
            />
          </div>
        </>
      ) : (
        <>
          <div className="intro">
            <p className="eyebrow">LEGAL OPERATIONS, WITH CLARITY</p>
            <h1>
              Clear requirements.
              <br />
              Confident next steps.
            </h1>
            <p className="summary">
              Keep matters organized, see what is missing, and understand the
              work ahead.
            </p>
            <DemoEntry busy={demo.busy} onEnter={demo.enter} />
            <ApiStatus />
          </div>
          <section className="modules" aria-label="Inside the demo">
            <article>
              <span>01</span>
              <h2>Matter visibility</h2>
              <p>
                Explore assignments and progress across a fictional caseload.
              </p>
            </article>
            <article>
              <span>02</span>
              <h2>Document checklists</h2>
              <p>
                Distinguish missing documents, received copies, and reviewed
                requirements.
              </p>
            </article>
            <article>
              <span>03</span>
              <h2>Team perspective</h2>
              <p>
                Switch to the reviewer role for a view of the whole workspace.
              </p>
            </article>
          </section>
        </>
      )}
      <footer>
        Independent portfolio demo. All clients, matters, and document
        requirements are fictional.
      </footer>
    </main>
  );
}
