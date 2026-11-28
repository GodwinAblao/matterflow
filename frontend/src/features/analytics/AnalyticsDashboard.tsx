import { useEffect, useState } from "react";
import { ApiError, apiGet } from "../../lib/api";
type Matter = {
  id: string;
  reference: string;
  title: string;
  status: string;
  assigned_to: string | null;
  required_documents: number;
  accepted_documents: number;
  days_to_readiness: number | null;
};
type Requirement = {
  id: string;
  matter_id: string;
  reference: string;
  title: string;
  status: string;
  age_days: number;
};
type FollowUp = {
  id: string;
  matter_id: string;
  reference: string;
  title: string;
  due_on: string;
  assigned_to: string | null;
  completed: boolean;
  matter_status: string;
  overdue: boolean;
};
type Report = {
  as_of: string;
  local_date: string;
  metrics: {
    visible_matters: number;
    active_matters: number;
    required_documents: number;
    accepted_documents: number;
    completeness_percent: number | null;
    outstanding_requirements: number;
    overdue_follow_ups: number;
    median_days_to_readiness: number | null;
    readiness_cohort: number;
  };
  matters: Matter[];
  requirements: Requirement[];
  follow_ups: FollowUp[];
};
type Props = {
  token: string;
  onExpired: () => void;
  onOpenMatter: (id: string) => void;
};
type Selection = {
  kind: "matters" | "requirements" | "followups";
  filter: string;
  label: string;
};
const label = (value: string) => value.replaceAll("_", " ");
function formatDays(value: number | null) {
  if (value === null) return "N/A";
  const days = Math.round(value);
  return `${days} ${days === 1 ? "day" : "days"}`;
}

export function AnalyticsDashboard(props: Props) {
  const [category, setCategory] = useState("");
  return (
    <section className="analytics">
      <div className="section-heading">
        <div>
          <p className="eyebrow">DESCRIPTIVE ANALYTICS</p>
          <h1>Work in perspective</h1>
          <p className="muted">
            Explore the records behind your workload and document progress.
          </p>
        </div>
        <label className="category-filter">
          Service category
          <select
            value={category}
            onChange={(event) => setCategory(event.target.value)}
          >
            <option value="">All categories</option>
            {["property", "business", "family", "criminal", "other"].map(
              (value) => (
                <option key={value} value={value}>
                  {label(value)}
                </option>
              ),
            )}
          </select>
        </label>
      </div>
      <ReportView key={category} {...props} category={category} />
    </section>
  );
}
function ReportView({
  token,
  onExpired,
  onOpenMatter,
  category,
}: Props & { category: string }) {
  const [report, setReport] = useState<Report | null>(null);
  const [error, setError] = useState("");
  const [attempt, setAttempt] = useState(0);
  const [loading, setLoading] = useState(true);
  const [selection, setSelection] = useState<Selection>({
    kind: "matters",
    filter: "all",
    label: "All visible matters",
  });
  useEffect(() => {
    const controller = new AbortController();
    void apiGet<Report>(
      "/analytics" +
        (category ? "?category=" + encodeURIComponent(category) : ""),
      controller.signal,
      token,
    )
      .then((data) => {
        if (!controller.signal.aborted) {
          setReport(data);
          setError("");
        }
      })
      .catch((failure: unknown) => {
        if (controller.signal.aborted) return;
        if (failure instanceof ApiError && failure.status === 401) onExpired();
        else
          setError(
            failure instanceof Error
              ? failure.message
              : "Could not load analytics.",
          );
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    return () => controller.abort();
  }, [token, category, onExpired, attempt]);
  const refresh = () => {
    setLoading(true);
    setAttempt((value) => value + 1);
  };
  if (!report)
    return error ? (
      <div className="notice error" role="alert">
        {error}
        <button onClick={refresh}>Retry</button>
      </div>
    ) : (
      <p className="loading" role="status">
        Calculating your report...
      </p>
    );
  const m = report.metrics;
  const stages = [
    "intake",
    "collecting_documents",
    "under_review",
    "ready",
  ].map((stage) => ({
    name: stage,
    count: report.matters.filter((matter) => matter.status === stage).length,
  }));
  const ages = [
    { name: "0-7 days", key: "0-7", test: (age: number) => age <= 7 },
    {
      name: "8-14 days",
      key: "8-14",
      test: (age: number) => age >= 8 && age <= 14,
    },
    { name: "15+ days", key: "15+", test: (age: number) => age >= 15 },
  ].map((bucket) => ({
    ...bucket,
    count: report.requirements.filter((row) => bucket.test(row.age_days))
      .length,
  }));
  const owners = [
    ...new Set(
      report.matters.map((matter) => matter.assigned_to ?? "Unassigned"),
    ),
  ].map((owner) => ({
    name: owner,
    count: report.matters.filter(
      (matter) =>
        matter.status !== "closed" &&
        (matter.assigned_to ?? "Unassigned") === owner,
    ).length,
  }));
  const matters = report.matters.filter((matter) =>
    selection.filter === "active"
      ? matter.status !== "closed"
      : selection.filter === "cohort"
        ? matter.days_to_readiness !== null
        : selection.filter.startsWith("stage:")
          ? matter.status === selection.filter.slice(6)
          : selection.filter.startsWith("owner:")
            ? matter.status !== "closed" &&
              (matter.assigned_to ?? "Unassigned") === selection.filter.slice(6)
            : true,
  );
  const requirements = report.requirements.filter((row) =>
    selection.filter === "0-7"
      ? row.age_days <= 7
      : selection.filter === "8-14"
        ? row.age_days >= 8 && row.age_days <= 14
        : selection.filter === "15+"
          ? row.age_days >= 15
          : true,
  );
  const followups = report.follow_ups.filter((row) =>
    selection.filter === "overdue" ? row.overdue : true,
  );
  const choose = (kind: Selection["kind"], filter: string, title: string) =>
    setSelection({ kind, filter, label: title });
  function exportCsv() {
    const rows =
      selection.kind === "matters"
        ? [
            [
              "Reference",
              "Matter",
              "Stage",
              "Assignee",
              "Accepted required documents",
              "Required documents",
              "Days to first readiness",
            ],
            ...matters.map((row) => [
              row.reference,
              row.title,
              row.status,
              row.assigned_to ?? "Unassigned",
              row.accepted_documents,
              row.required_documents,
              row.days_to_readiness ?? "",
            ]),
          ]
        : selection.kind === "requirements"
          ? [
              ["Reference", "Requirement", "Status", "Age in whole days"],
              ...requirements.map((row) => [
                row.reference,
                row.title,
                row.status,
                row.age_days,
              ]),
            ]
          : [
              [
                "Reference",
                "Follow-up",
                "Due date",
                "Assignee",
                "Completed",
                "Overdue",
              ],
              ...followups.map((row) => [
                row.reference,
                row.title,
                row.due_on,
                row.assigned_to ?? "Unassigned",
                row.completed,
                row.overdue,
              ]),
            ];
    const text = rows
      .map((row) =>
        row
          .map((cell) => {
            let value = String(cell);
            if (/^[\s]*[=+@\-\t\r]/.test(value)) value = "'" + value;
            return '"' + value.replaceAll('"', '""') + '"';
          })
          .join(","),
      )
      .join("\r\n");
    const url = URL.createObjectURL(
      new Blob([text], { type: "text/csv;charset=utf-8;" }),
    );
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = "matterflow-analytics.csv";
    anchor.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
  return (
    <>
      <div className="report-toolbar">
        <p className="muted small">
          Snapshot{" "}
          {new Intl.DateTimeFormat("en-PH", {
            dateStyle: "medium",
            timeStyle: "short",
            timeZone: "Asia/Manila",
          }).format(new Date(report.as_of))}{" "}
          PH time. All records are fictional.
        </p>
        <button disabled={loading} onClick={refresh}>
          {loading ? "Refreshing..." : "Refresh report"}
        </button>
      </div>
      {error && (
        <p className="notice error" role="alert">
          {error}
        </p>
      )}
      <div className="metric-grid" aria-label="Analytics metrics">
        <button onClick={() => choose("matters", "all", "All visible matters")}>
          <span>Visible matters</span>
          <strong>{m.visible_matters}</strong>
          <small>Includes closed matters</small>
        </button>
        <button onClick={() => choose("matters", "active", "Active matters")}>
          <span>Active matters</span>
          <strong>{m.active_matters}</strong>
          <small>Every stage except closed</small>
        </button>
        <button
          onClick={() =>
            choose("matters", "all", "Document completeness by matter")
          }
        >
          <span>Required document completeness</span>
          <strong>
            {m.completeness_percent === null
              ? "N/A"
              : m.completeness_percent.toFixed(1) + "%"}
          </strong>
          <small>
            {m.accepted_documents} accepted of {m.required_documents} required
          </small>
        </button>
        <button
          onClick={() => choose("followups", "overdue", "Overdue follow-ups")}
        >
          <span>Overdue follow-ups</span>
          <strong>{m.overdue_follow_ups}</strong>
          <small>Open tasks due before today</small>
        </button>
        <button
          onClick={() =>
            choose("requirements", "all", "Outstanding required documents")
          }
        >
          <span>Outstanding requirements</span>
          <strong>{m.outstanding_requirements}</strong>
          <small>Unaccepted required documents</small>
        </button>
        <button
          onClick={() =>
            choose("matters", "cohort", "Matters that reached readiness")
          }
        >
          <span>Median time to first readiness</span>
          <strong>{formatDays(m.median_days_to_readiness)}</strong>
          <small>{m.readiness_cohort} matters reached readiness</small>
        </button>
      </div>
      <div className="chart-grid">
        <section className="panel chart-panel">
          <h2>Active matters by stage</h2>
          <p className="muted small">Select a stage to inspect its matters.</p>
          {stages.map((stage) => (
            <button
              className="bar-row"
              key={stage.name}
              onClick={() =>
                choose(
                  "matters",
                  "stage:" + stage.name,
                  label(stage.name) + " matters",
                )
              }
            >
              <span>{label(stage.name)}</span>
              <span className="bar-track">
                <span
                  style={{
                    width:
                      (stage.count /
                        Math.max(1, ...stages.map((row) => row.count))) *
                        100 +
                      "%",
                  }}
                />
              </span>
              <strong>{stage.count}</strong>
            </button>
          ))}
        </section>
        <section className="panel chart-panel">
          <h2>Outstanding requirement age</h2>
          <p className="muted small">Whole days since the original request.</p>
          {ages.map((bucket) => (
            <button
              className="bar-row"
              key={bucket.key}
              onClick={() =>
                choose(
                  "requirements",
                  bucket.key,
                  bucket.name + " outstanding requirements",
                )
              }
            >
              <span>{bucket.name}</span>
              <span className="bar-track age">
                <span
                  style={{
                    width:
                      (bucket.count /
                        Math.max(1, ...ages.map((row) => row.count))) *
                        100 +
                      "%",
                  }}
                />
              </span>
              <strong>{bucket.count}</strong>
            </button>
          ))}
        </section>
        <section className="panel chart-panel">
          <h2>Open assignments</h2>
          <p className="muted small">
            Matter counts describe distribution, not productivity.
          </p>
          {owners.map((owner) => (
            <button
              className="bar-row owner"
              key={owner.name}
              onClick={() =>
                choose(
                  "matters",
                  "owner:" + owner.name,
                  owner.name + " active matters",
                )
              }
            >
              <span>{owner.name}</span>
              <span className="bar-track">
                <span
                  style={{
                    width:
                      (owner.count /
                        Math.max(1, ...owners.map((row) => row.count))) *
                        100 +
                      "%",
                  }}
                />
              </span>
              <strong>{owner.count}</strong>
            </button>
          ))}
          {owners.length === 0 && (
            <p className="muted small">No assignments in this category.</p>
          )}
        </section>
      </div>
      <section className="panel drilldown">
        <div className="panel-heading">
          <div>
            <p className="eyebrow">UNDERLYING RECORDS</p>
            <h2>{selection.label}</h2>
          </div>
          <div className="document-actions">
            <button
              onClick={() =>
                choose("followups", "all", "All visible follow-ups")
              }
            >
              All follow-ups
            </button>
            <button onClick={exportCsv}>Export CSV</button>
          </div>
        </div>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th>Reference</th>
                <th>
                  {selection.kind === "matters"
                    ? "Matter"
                    : selection.kind === "requirements"
                      ? "Requirement"
                      : "Follow-up"}
                </th>
                <th>
                  {selection.kind === "followups" ? "Due date" : "Status"}
                </th>
                <th>
                  {selection.kind === "matters"
                    ? "Required accepted"
                    : selection.kind === "requirements"
                      ? "Age"
                      : "State"}
                </th>
              </tr>
            </thead>
            <tbody>
              {selection.kind === "matters" &&
                matters.map((row) => (
                  <tr key={row.id}>
                    <td>
                      <button
                        className="text-button"
                        onClick={() => onOpenMatter(row.id)}
                      >
                        {row.reference}
                      </button>
                    </td>
                    <td>
                      {row.title}
                      {selection.filter === "cohort" && (
                        <p className="muted small">
                          {formatDays(row.days_to_readiness)} to first readiness
                        </p>
                      )}
                    </td>
                    <td>
                      <span className={"stage " + row.status}>
                        {label(row.status)}
                      </span>
                    </td>
                    <td>
                      {row.accepted_documents} / {row.required_documents}
                    </td>
                  </tr>
                ))}
              {selection.kind === "requirements" &&
                requirements.map((row) => (
                  <tr key={row.id}>
                    <td>
                      <button
                        className="text-button"
                        onClick={() => onOpenMatter(row.matter_id)}
                      >
                        {row.reference}
                      </button>
                    </td>
                    <td>{row.title}</td>
                    <td>
                      <span className={"stage " + row.status}>
                        {label(row.status)}
                      </span>
                    </td>
                    <td>{row.age_days} days</td>
                  </tr>
                ))}
              {selection.kind === "followups" &&
                followups.map((row) => (
                  <tr key={row.id}>
                    <td>
                      <button
                        className="text-button"
                        onClick={() => onOpenMatter(row.matter_id)}
                      >
                        {row.reference}
                      </button>
                    </td>
                    <td>
                      {row.title}
                      <p className="muted small">
                        {row.assigned_to ?? "Unassigned"}
                      </p>
                    </td>
                    <td>{row.due_on}</td>
                    <td>
                      <span
                        className={
                          "stage " +
                          (row.overdue
                            ? "needs_correction"
                            : row.completed
                              ? "accepted"
                              : "received")
                        }
                      >
                        {row.completed
                          ? "Completed"
                          : row.overdue
                            ? "Overdue"
                            : "Outstanding"}
                      </span>
                    </td>
                  </tr>
                ))}
            </tbody>
          </table>
        </div>
        {(selection.kind === "matters"
          ? matters.length
          : selection.kind === "requirements"
            ? requirements.length
            : followups.length) === 0 && (
          <p className="empty">No records match this selection.</p>
        )}
      </section>
      <details className="metric-definitions">
        <summary>How these metrics are calculated</summary>
        <ul>
          <li>
            Scope follows your role and selected category. Staff task metrics
            include their own assigned follow-ups.
          </li>
          <li>
            Completeness includes required documents across all visible matters,
            including closed matters. An empty denominator displays N/A.
          </li>
          <li>
            Outstanding requirements exclude accepted documents and closed
            matters. Age uses elapsed 24-hour periods from the original request,
            including corrections awaiting resolution.
          </li>
          <li>
            Overdue means an unfinished follow-up on a nonclosed matter with a
            due date before {report.local_date}, using the Philippines calendar.
            Tasks due today are not overdue.
          </li>
          <li>
            Median readiness time includes only matters with a recorded first
            readiness, including closed matters. Unfinished matters are excluded
            from that metric and remain visible in the aging report. Readiness
            durations are displayed rounded to the nearest whole day;
            calculations and CSV exports retain their original precision.
          </li>
        </ul>
      </details>
    </>
  );
}
