import { NewMatterForm } from "./NewMatterForm";
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
  received_documents?: number;
};
type TeamMember = {
  display_name: string;
  role: string;
  active_matters: number;
};
type Props = { token: string; session: DemoSession; onExpired: () => void };

const label = (value: string) => value.replaceAll("_", " ");

export function SessionWorkspace(props: Props) {
  const [selectedMatterId, setSelectedMatterId] = useState<string | null>(null);
  return (
    <DemoWorkspace
      key={props.session.role}
      {...props}
      selectedMatterId={selectedMatterId}
      onSelectMatter={setSelectedMatterId}
    />
  );
}

type WorkspaceProps = Props & {
  selectedMatterId: string | null;
  onSelectMatter: (id: string | null) => void;
};
function DemoWorkspace({
  token,
  session,
  onExpired,
  selectedMatterId,
  onSelectMatter,
}: WorkspaceProps) {
  const [matters, setMatters] = useState<Matter[] | null>(null);
  const [team, setTeam] = useState<TeamMember[]>([]);
  const [error, setError] = useState("");
  const [attempt, setAttempt] = useState(0);
  const [search, setSearch] = useState("");
  const [pageSize, setPageSize] = useState(5);
  const [requestedPage, setRequestedPage] = useState<number | null>(null);
  const [creating, setCreating] = useState(false);
  const [createdNotice, setCreatedNotice] = useState("");
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
        if (controller.signal.aborted) return;
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
  const selected = matters.some((matter) => matter.id === selectedMatterId)
    ? selectedMatterId
    : null;
  const selectionUnavailable = selectedMatterId !== null && selected === null;
  const visible = matters.filter((matter) =>
    (matter.reference + " " + matter.title + " " + matter.client_label)
      .toLowerCase()
      .includes(search.toLowerCase()),
  );
  const pageCount = Math.max(1, Math.ceil(visible.length / pageSize));
  const selectedIndex = visible.findIndex((matter) => matter.id === selected);
  const selectionPage =
    selectedIndex < 0 ? 1 : Math.floor(selectedIndex / pageSize) + 1;
  const page = Math.min(pageCount, Math.max(1, requestedPage ?? selectionPage));
  const pageStart = (page - 1) * pageSize;
  const pageRows = visible.slice(pageStart, pageStart + pageSize);
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
            onSelectMatter(id);
            setSearch("");
            setRequestedPage(null);
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
        <div className="matter-header-actions">
          <span className="badge">Fictional demo data</span>
          <button
            className="primary"
            onClick={() => setCreating((value) => !value)}
          >
            {creating ? "Close form" : "New matter"}
          </button>
        </div>
      </div>
      {selectionUnavailable && (
        <p className="notice selection-notice" role="status">
          The previously selected matter is not available in this role. Choose a
          matter assigned to you below.
        </p>
      )}
      {createdNotice && (
        <p className="notice success" role="status">
          Created {createdNotice}. Its checklist is open below.{" "}
          <button onClick={() => setCreatedNotice("")}>Dismiss</button>
        </p>
      )}
      {creating && (
        <NewMatterForm
          token={token}
          role={session.role}
          onExpired={onExpired}
          onCancel={() => setCreating(false)}
          onCreated={(id, reference) => {
            setCreating(false);
            onSelectMatter(id);
            setSearch("");
            setRequestedPage(null);
            setCreatedNotice(reference);
            onChanged();
          }}
        />
      )}
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
                onChange={(event) => {
                  setSearch(event.target.value);
                  setRequestedPage(1);
                }}
              />
            </label>
          </div>
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th scope="col">Matter</th>
                  <th scope="col">Stage</th>
                  <th scope="col">Required documents accepted</th>
                </tr>
              </thead>
              <tbody>
                {pageRows.map((matter) => (
                  <tr
                    key={matter.id}
                    className={selected === matter.id ? "row-selected" : ""}
                  >
                    <td>
                      <button
                        className="text-button"
                        aria-label={"Open " + matter.reference}
                        onClick={() => onSelectMatter(matter.id)}
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
                      {matter.received_documents !== undefined && (
                        <p
                          className="receipt-count muted small"
                          title="Required documents with a recorded receipt, including accepted documents and documents needing correction."
                        >
                          Received: {matter.received_documents} /{" "}
                          {matter.required_documents}
                        </p>
                      )}
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
          <div className="register-pagination">
            <p className="muted small" role="status" aria-live="polite">
              {visible.length === 0
                ? "No matching matters"
                : "Showing " +
                  (pageStart + 1) +
                  "-" +
                  Math.min(pageStart + pageSize, visible.length) +
                  " of " +
                  visible.length +
                  " matters"}
            </p>
            <label className="page-size">
              Rows per page
              <select
                aria-label="Matters per page"
                value={pageSize}
                onChange={(event) => {
                  setPageSize(Number(event.target.value));
                  setRequestedPage(1);
                }}
              >
                {[5, 10, 20].map((size) => (
                  <option key={size} value={size}>
                    {size}
                  </option>
                ))}
              </select>
            </label>
            <nav aria-label="Matter register pagination">
              <button
                aria-label="Previous matter page"
                disabled={page <= 1}
                onClick={() => setRequestedPage(page - 1)}
              >
                Previous
              </button>
              <span>
                Page {page} of {pageCount}
              </span>
              <button
                aria-label="Next matter page"
                disabled={page >= pageCount}
                onClick={() => setRequestedPage(page + 1)}
              >
                Next
              </button>
            </nav>
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
              onDeleted={() => {
                onSelectMatter(null);
                setCreatedNotice("Draft deleted.");
              }}
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
