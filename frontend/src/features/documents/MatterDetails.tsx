import { displayLabel as label } from "../../lib/displayLabel";
import { MatterManagement } from "../matters/MatterManagement";
import { NewRequirementForm } from "./NewRequirementForm";
import { useEffect, useState } from "react";
import { ApiError, apiGet, apiRequest } from "../../lib/api";
import { FollowUpPanel } from "../follow-ups/FollowUpPanel";
import type { FollowUp } from "../follow-ups/FollowUpPanel";
import type { Role } from "../auth/useDemoSession";

type Requirement = {
  id: string;
  title: string;
  is_required: boolean;
  status: string;
  version: number;
  correction_note: string | null;
  received_at: string | null;
  reviewed_at: string | null;
};
type Blocker = {
  requirement_id: string | null;
  title: string;
  status: string;
  reason: string;
};
type Activity = {
  id: string;
  event_type: string;
  actor_name: string | null;
  occurred_at: string;
  details: {
    document_title?: string;
    follow_up_title?: string;
    note?: string;
    from_status?: string;
    to_status?: string;
  };
};
type Detail = {
  matter: {
    id: string;
    reference: string;
    title: string;
    assigned_to: string | null;
    assigned_staff_id: string | null;
    client_label: string;
    service_category: string;
    status: string;
    version: number;
  };
  can_delete: boolean;
  documents: Requirement[];
  follow_ups: FollowUp[];
  readiness: { documents_complete: boolean; blockers: Blocker[] };
  activity: Activity[];
};
type Props = {
  id: string;
  token: string;
  role: Role;
  onExpired: () => void;
  onChanged: () => void;
  onDeleted?: () => void;
};

const timestamp = (value: string) =>
  new Intl.DateTimeFormat("en-PH", {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: "Asia/Manila",
  }).format(new Date(value));
const events: Record<string, string> = {
  "matter.opened": "Matter opened",
  "matter.updated": "Matter details updated",
  "matter.archived": "Matter archived",
  "document.received": "Document received",
  "document.resubmitted": "Corrected document received",
  "document.accepted": "Document approved",
  "document.correction_requested": "Correction requested",
  "matter.ready": "Marked ready",
  "requirement.added": "Checklist requirement added",
  "follow_up.created": "Follow-up created",
  "follow_up.completed": "Follow-up completed",
  "follow_up.reopened": "Follow-up reopened",
  "follow_up.rescheduled": "Follow-up rescheduled",
};

export function MatterDetails({
  id,
  token,
  role,
  onExpired,
  onChanged,
  onDeleted,
}: Props) {
  const [detail, setDetail] = useState<Detail | null>(null);
  const [loadError, setLoadError] = useState("");
  const [actionError, setActionError] = useState("");
  const [success, setSuccess] = useState("");
  const [attempt, setAttempt] = useState(0);
  const [busy, setBusy] = useState(false);
  const [correction, setCorrection] = useState<string | null>(null);
  const [note, setNote] = useState("");
  useEffect(() => {
    const controller = new AbortController();
    void apiGet<Detail>("/matters/" + id, controller.signal, token)
      .then((data) => {
        if (!controller.signal.aborted) {
          setDetail(data);
          setLoadError("");
        }
      })
      .catch((failure: unknown) => {
        if (controller.signal.aborted) return;
        if (failure instanceof ApiError && failure.status === 401) onExpired();
        else
          setLoadError(
            failure instanceof Error
              ? failure.message
              : "Could not load this matter.",
          );
      });
    return () => controller.abort();
  }, [id, token, onExpired, attempt]);

  async function reload() {
    setDetail(await apiGet<Detail>("/matters/" + id, undefined, token));
    setLoadError("");
  }
  async function change(
    path: string,
    body: unknown,
    message: string,
    method: "POST" | "PATCH" = "POST",
  ): Promise<boolean> {
    setBusy(true);
    setActionError("");
    setSuccess("");
    try {
      await apiRequest("/matters/" + id + path, {
        method,
        body,
        token,
      });
      await reload();
      onChanged();
      setCorrection(null);
      setNote("");
      setSuccess(message);
      return true;
    } catch (failure) {
      if (failure instanceof ApiError && failure.status === 401) {
        onExpired();
        return false;
      }
      setActionError(
        failure instanceof Error
          ? failure.message
          : "Could not save this change.",
      );
      if (failure instanceof ApiError && failure.status === 409) {
        try {
          await reload();
          onChanged();
        } catch {
          setActionError(
            "This record changed. Refresh the matter before trying again.",
          );
        }
      }
      return false;
    } finally {
      setBusy(false);
    }
  }
  if (!detail)
    return loadError ? (
      <div className="notice error" role="alert">
        {loadError}
        <button onClick={() => setAttempt((value) => value + 1)}>Retry</button>
      </div>
    ) : (
      <p role="status" className="loading">
        Loading matter details...
      </p>
    );
  const locked = ["ready", "closed"].includes(detail.matter.status);
  return (
    <div aria-busy={busy}>
      <div className="detail-heading">
        <p className="eyebrow">{detail.matter.reference}</p>
        <h2>{detail.matter.title}</h2>
        <span className={"stage " + detail.matter.status}>
          {label(detail.matter.status)}
        </span>
        <p className="muted small">
          Assigned to {detail.matter.assigned_to ?? "Unassigned"}
        </p>
      </div>
      {loadError && (
        <p className="notice error workflow-notice" role="alert">
          {loadError}{" "}
          <button onClick={() => setAttempt((value) => value + 1)}>
            Retry refresh
          </button>
        </p>
      )}
      <MatterManagement
        matter={detail.matter}
        canDelete={detail.can_delete}
        token={token}
        role={role}
        busy={busy}
        onBusy={setBusy}
        onExpired={onExpired}
        onSaved={async () => {
          await reload();
          onChanged();
        }}
        onDeleted={() => {
          onDeleted?.();
          onChanged();
        }}
      />
      {actionError && (
        <p className="notice error workflow-notice" role="alert">
          {actionError}
        </p>
      )}
      {success && (
        <p className="notice success workflow-notice" role="status">
          {success}
        </p>
      )}
      {busy && (
        <p className="muted small workflow-notice" role="status">
          Saving change...
        </p>
      )}
      <section
        className="readiness-panel"
        aria-label="Are the documents ready?"
      >
        <div className="readiness-heading">
          <h3>Are the documents ready?</h3>
          <button
            className="text-button"
            disabled={busy}
            onClick={() => {
              setSuccess("");
              setAttempt((value) => value + 1);
            }}
          >
            Refresh check
          </button>
        </div>
        {detail.matter.status === "closed" ? (
          <p className="small muted">
            This matter is closed. Document changes are unavailable.
          </p>
        ) : detail.matter.status === "ready" ? (
          <p className="readiness-complete">
            Ready for the next step. All required documents have been approved.
          </p>
        ) : detail.readiness.documents_complete ? (
          <p className="readiness-complete">
            All required documents are approved. A reviewer can mark this matter
            ready.
          </p>
        ) : (
          <>
            <p className="small muted">
              These documents still need approval before the matter can be
              marked ready:
            </p>
            <ul className="blockers">
              {detail.readiness.blockers.map((blocker) => (
                <li key={blocker.requirement_id ?? "empty"}>
                  <strong>{blocker.title}</strong>
                  <span>{blocker.reason}</span>
                </li>
              ))}
            </ul>
          </>
        )}
        {role === "reviewer" && !locked && (
          <button
            className="primary readiness-button"
            disabled={busy || !detail.readiness.documents_complete}
            onClick={() =>
              void change(
                "/ready",
                { expected_version: detail.matter.version },
                "Matter marked ready. The activity history has been updated.",
              )
            }
          >
            Mark matter ready
          </button>
        )}
        {role === "staff" && !locked && (
          <p className="small muted">
            A reviewer can mark this matter ready after approving every required
            document.
          </p>
        )}
      </section>
      <h3>Document checklist</h3>
      {!locked && detail.documents.length < 30 && (
        <NewRequirementForm
          version={detail.matter.version}
          busy={busy}
          onChange={change}
        />
      )}
      <ul className="checklist">
        {detail.documents.map((document) => (
          <li key={document.id}>
            <div className="checklist-title">
              <strong>{document.title}</strong>
              <small className="muted">
                {document.is_required ? "Required" : "Optional"}
              </small>
            </div>
            <span className={"stage " + document.status}>
              {label(document.status)}
            </span>
            {document.received_at && (
              <p className="document-time">
                Received {timestamp(document.received_at)} PH time
              </p>
            )}
            {document.reviewed_at && (
              <p className="document-time">
                Reviewed {timestamp(document.reviewed_at)} PH time
              </p>
            )}
            {document.correction_note && (
              <p className="correction-note">{document.correction_note}</p>
            )}
            {!locked &&
              role === "staff" &&
              ["missing", "needs_correction"].includes(document.status) && (
                <div className="document-actions">
                  <button
                    disabled={busy}
                    onClick={() =>
                      void change(
                        "/documents/" + document.id + "/receive",
                        { expected_version: document.version },
                        document.status === "needs_correction"
                          ? "Corrected receipt recorded. The document is awaiting review."
                          : "Receipt recorded. The document is awaiting review.",
                      )
                    }
                  >
                    {document.status === "needs_correction"
                      ? "Record corrected document"
                      : "Record document received"}
                  </button>
                </div>
              )}
            {!locked &&
              role === "reviewer" &&
              document.status === "received" && (
                <>
                  <div className="document-actions">
                    <button
                      className="accept-button"
                      disabled={busy}
                      onClick={() =>
                        void change(
                          "/documents/" + document.id + "/review",
                          {
                            expected_version: document.version,
                            outcome: "accepted",
                            note: null,
                          },
                          "Document approved.",
                        )
                      }
                    >
                      Approve document
                    </button>
                    <button
                      disabled={busy}
                      onClick={() => {
                        setCorrection(document.id);
                        setNote("");
                      }}
                    >
                      Request changes
                    </button>
                  </div>
                  {correction === document.id && (
                    <form
                      className="correction-form"
                      onSubmit={(event) => {
                        event.preventDefault();
                        void change(
                          "/documents/" + document.id + "/review",
                          {
                            expected_version: document.version,
                            outcome: "needs_correction",
                            note,
                          },
                          "Correction requested. Staff can record a corrected submission.",
                        );
                      }}
                    >
                      <label htmlFor={"correction-" + document.id}>
                        What needs to be corrected?
                      </label>
                      <textarea
                        id={"correction-" + document.id}
                        required
                        maxLength={1000}
                        rows={3}
                        value={note}
                        disabled={busy}
                        onChange={(event) => setNote(event.target.value)}
                      />
                      <div className="document-actions">
                        <button type="submit" disabled={busy || !note.trim()}>
                          Send correction request
                        </button>
                        <button
                          type="button"
                          disabled={busy}
                          onClick={() => {
                            setCorrection(null);
                            setNote("");
                          }}
                        >
                          Cancel
                        </button>
                      </div>
                    </form>
                  )}
                </>
              )}
          </li>
        ))}
      </ul>
      <FollowUpPanel
        rows={detail.follow_ups}
        closed={detail.matter.status === "closed"}
        busy={busy}
        onChange={change}
      />
      <h3>Activity history</h3>
      <ol className="activity-timeline">
        {detail.activity.map((activity) => (
          <li key={activity.id}>
            <strong>
              {events[activity.event_type] ?? label(activity.event_type)}
            </strong>
            {activity.details.follow_up_title && (
              <p>{activity.details.follow_up_title}</p>
            )}
            {activity.details.document_title && (
              <p>{activity.details.document_title}</p>
            )}
            {activity.details.note && (
              <p className="activity-note">{activity.details.note}</p>
            )}
            <small>
              {activity.actor_name ?? "System"} -{" "}
              {timestamp(activity.occurred_at)} PH time
            </small>
          </li>
        ))}
      </ol>
    </div>
  );
}
