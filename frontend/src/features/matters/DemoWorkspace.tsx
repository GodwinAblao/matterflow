import { AnalyticsDashboard } from "../analytics/AnalyticsDashboard";
import { useCallback, useEffect, useState } from "react";
import { ApiError, apiGet } from "../../lib/api";
import type { DemoSession } from "../auth/useDemoSession";
import { MatterDetails } from "../documents/MatterDetails";

type Matter = {
  id: string;
  reference: string;
  title: string;
  client_label: string;
  service_category: string;
  status: string;
  assigned_to: string | null;
  required_documents: number;
  accepted_documents: number;
};
type TeamMember = {
  display_name: string;
  role: string;
  active_matters: number;
};
type Props = { token: string; session: DemoSession; onExpired: () => void };

const label = (value: string) => value.replaceAll("_", " ");

export function DemoWorkspace({ token, session, onExpired }: Props) {
  const [matters, setMatters] = useState<Matter[] | null>(null);
  const [team, setTeam] = useState<TeamMember[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [attempt, setAttempt] = useState(0);
  const [search, setSearch] = useState("");
  const [view, setView] = useState<"matters" | "analytics">("matters");
  const onChanged = useCallback(() => setAttempt((value) => value + 1), []);

  useEffect(() => {
    const controller = new AbortController();
    void Promise.all([
      apiGet<Matter[]>("/matters", controller.signal, token),
      session.role === "reviewer"
        ? apiGet<TeamMember[]>("/team", controller.signal, token)
        : Promise.resolve([]),
    ])
      .then(([rows, members]) => {
        setMatters(rows);
        setTeam(members);
        setError("");
      })
      .catch((failure: unknown) => {
        if (controller.signal.aborted) return;
        if (failure instanceof ApiError && failure.status === 401) onExpired();
        else
          setError(
            failure instanceof Error
              ? failure.message
              : "Could not load the workspace.",
          );
      });
    return () => controller.abort();
  }, [token, session.role, onExpired, attempt]);

  if (!matters)
    return error ? (
      <div className="notice error" role="alert">
        {error}{" "}
        <button onClick={() => setAttempt((value) => value + 1)}>Retry</button>
      </div>
    ) : (
      <p role="status" className="loading">
        Loading your fictional matters...
      </p>
    );
  const visible = matters.filter((matter) =>
    (matter.reference + " " + matter.title + " " + matter.client_label)
      .toLowerCase()
      .includes(search.toLowerCase()),
  );
  const active = matters.filter((matter) => matter.status !== "closed").length;
  const required = matters.reduce(
    (sum, matter) => sum + matter.required_documents,
    0,
  );
  const accepted = matters.reduce(
    (sum, matter) => sum + matter.accepted_documents,
    0,
  );

  const navigation = (
    <nav className="workspace-nav" aria-label="Workspace sections">
      <button
        aria-pressed={view === "matters"}
        onClick={() => setView("matters")}
      >
        Matters
      </button>
      <button
        aria-pressed={view === "analytics"}
        onClick={() => setView("analytics")}
      >
        Analytics
      </button>
    </nav>
  );
  if (view === "analytics")
    return (
      <div className="workspace">
        {navigation}
        <AnalyticsDashboard
          token={token}
          onExpired={onExpired}
          onOpenMatter={(id) => {
            setSelected(id);
            setView("matters");
          }}
        />
      </div>
    );
  return (
    <div className="workspace">
      {navigation}
      {error && (
        <div role="alert" className="notice error">
          {error} <button onClick={onChanged}>Retry register refresh</button>
        </div>
      )}
      <div className="section-heading">
        <div>
          <p className="eyebrow">YOUR WORKSPACE</p>
          <h1>{session.role === "staff" ? "My matters" : "Matter overview"}</h1>
          <p className="muted">
            {session.role === "staff"
              ? "Matters assigned to you, with requirements at a glance."
              : "All matters in this demo, with a view of the team workload."}
          </p>
        </div>
        <span className="badge">Fictional demo data</span>
      </div>
      <section className="stat-grid" aria-label="Visible matter summary">
        <article>
          <span>Visible matters</span>
          <strong>{matters.length}</strong>
        </article>
        <article>
          <span>Active matters</span>
          <strong>{active}</strong>
        </article>
        <article>
          <span>Required documents accepted</span>
          <strong>
            {accepted}
            <small> / {required}</small>
          </strong>
        </article>
      </section>
      <div className="work-grid">
        <section className="panel matters-panel" aria-label="Matter register">
          <div className="panel-heading">
            <h2>Matter register</h2>
            <label className="search">
              <span className="sr-only">Search matters</span>
              <input
                type="search"
                placeholder="Search matters..."
                value={search}
                onChange={(event) => setSearch(event.target.value)}
              />
            </label>
          </div>
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th scope="col">Matter</th>
                  <th scope="col">Stage</th>
                  <th scope="col">Required documents</th>
                </tr>
              </thead>
              <tbody>
                {visible.map((matter) => (
                  <tr
                    key={matter.id}
                    className={selected === matter.id ? "row-selected" : ""}
                  >
                    <td>
                      <button
                        className="text-button"
                        aria-label={"Open " + matter.reference}
                        onClick={() => setSelected(matter.id)}
                      >
                        {matter.reference}
                      </button>
                      <p className="matter-title">{matter.title}</p>
                      <small className="muted">{matter.client_label}</small>
                    </td>
                    <td>
                      <span className={"stage " + matter.status}>
                        {label(matter.status)}
                      </span>
                    </td>
                    <td>
                      <strong>
                        {matter.accepted_documents} /{" "}
                        {matter.required_documents}
                      </strong>
                      <div
                        className="progress"
                        role="progressbar"
                        aria-label={
                          matter.reference + " required documents accepted"
                        }
                        aria-valuenow={matter.accepted_documents}
                        aria-valuemin={0}
                        aria-valuemax={matter.required_documents || 1}
                      >
                        <span
                          style={{
                            width: matter.required_documents
                              ? (matter.accepted_documents /
                                  matter.required_documents) *
                                  100 +
                                "%"
                              : "0%",
                          }}
                        />
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {visible.length === 0 && (
            <p className="empty">No matters match your search.</p>
          )}
        </section>
        <aside className="panel detail-panel" aria-label="Matter details">
          {selected ? (
            <MatterDetails
              key={selected}
              id={selected}
              token={token}
              onExpired={onExpired}
              role={session.role}
              onChanged={onChanged}
            />
          ) : (
            <div className="empty detail-empty">
              <svg
                className="document-icon"
                aria-hidden="true"
                width="40"
                height="48"
                viewBox="0 0 40 48"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
              >
                <path d="M8 3h16l9 9v32H8zM24 3v10h9M14 22h13M14 29h13M14 36h9" />
              </svg>
              <h2>A closer look</h2>
              <p>
                Select a matter to see its document checklist and follow-ups.
              </p>
            </div>
          )}
        </aside>
      </div>
      {session.role === "reviewer" && (
        <section className="panel team-panel">
          <div className="panel-heading">
            <div>
              <h2>Team workload</h2>
              <p className="muted small">
                Open assignments across your demo workspace.
              </p>
            </div>
            <span className="badge">Reviewer view</span>
          </div>
          <div className="team-grid">
            {team
              .filter((member) => member.role === "staff")
              .map((member) => (
                <article key={member.display_name}>
                  <span className="avatar" aria-hidden="true">
                    {member.display_name.charAt(0)}
                  </span>
                  <div>
                    <strong>{member.display_name}</strong>
                    <p className="muted small">
                      {member.active_matters} active matters
                    </p>
                  </div>
                </article>
              ))}
          </div>
        </section>
      )}
    </div>
  );
}
