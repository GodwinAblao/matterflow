import { useEffect, useState } from "react";
import { ApiError, apiGet, apiRequest } from "../../lib/api";
import type { Role } from "../auth/useDemoSession";

type Matter = {
  id: string;
  reference: string;
  title: string;
  client_label: string;
  service_category: string;
  assigned_staff_id: string | null;
  status: string;
  version: number;
};
type Props = {
  matter: Matter;
  canDelete: boolean;
  role: Role;
  token: string;
  busy: boolean;
  onBusy: (busy: boolean) => void;
  onSaved: () => Promise<void>;
  onDeleted: () => void;
  onExpired: () => void;
};
export function MatterManagement({
  matter,
  canDelete,
  role,
  token,
  busy,
  onBusy,
  onSaved,
  onDeleted,
  onExpired,
}: Props) {
  const [mode, setMode] = useState<"edit" | "delete" | "archive" | null>(null);
  const [title, setTitle] = useState(matter.title);
  const [client, setClient] = useState(matter.client_label);
  const [category, setCategory] = useState(matter.service_category);
  const [assigned, setAssigned] = useState(matter.assigned_staff_id ?? "");
  const [staff, setStaff] = useState<{ id: string; display_name: string }[]>(
    [],
  );
  const [confirmed, setConfirmed] = useState("");
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const editable = !["ready", "closed"].includes(matter.status);
  useEffect(() => {
    if (mode !== "edit") return;
    const controller = new AbortController();
    void apiGet<typeof staff>("/matters/options", controller.signal, token)
      .then((rows) => {
        if (!controller.signal.aborted) setStaff(rows);
      })
      .catch(() => {
        if (!controller.signal.aborted)
          setError(
            "Could not load assignments. Close and reopen Edit matter to try again.",
          );
      });
    return () => controller.abort();
  }, [mode, token]);
  function open(next: typeof mode) {
    setMode(next);
    setError("");
    setNotice("");
    setConfirmed("");
    setTitle(matter.title);
    setClient(matter.client_label);
    setCategory(matter.service_category);
    setAssigned(matter.assigned_staff_id ?? "");
    setStaff([]);
  }
  async function submit() {
    if (!mode || busy) return;
    onBusy(true);
    setError("");
    setNotice("");
    try {
      await apiRequest(
        "/matters/" + matter.id + (mode === "archive" ? "/archive" : ""),
        {
          method:
            mode === "edit" ? "PATCH" : mode === "delete" ? "DELETE" : "POST",
          token,
          body:
            mode === "edit"
              ? {
                  expected_version: matter.version,
                  title,
                  client_label: client,
                  service_category: category,
                  assigned_staff_id: assigned,
                }
              : { expected_version: matter.version, reference: confirmed },
        },
      );
      if (mode === "delete") {
        onDeleted();
        return;
      }
      setMode(null);
      await onSaved();
      setNotice(
        mode === "edit"
          ? "Matter updated."
          : "Matter archived as Closed. Its records and history are preserved.",
      );
    } catch (failure) {
      if (failure instanceof ApiError && failure.status === 401) onExpired();
      else {
        setError(
          failure instanceof Error
            ? failure.message
            : "Could not save this change.",
        );
        if (failure instanceof ApiError && failure.status === 409) {
          setMode(null);
          await onSaved().catch(() => {});
        }
      }
    } finally {
      onBusy(false);
    }
  }
  return (
    <section className="matter-management" aria-label="Manage matter">
      <div className="document-actions">
        {editable && (
          <button disabled={busy || mode !== null} onClick={() => open("edit")}>
            Edit matter
          </button>
        )}
        {canDelete && (
          <button
            disabled={busy || mode !== null}
            onClick={() => open("delete")}
          >
            Delete draft
          </button>
        )}
        {role === "reviewer" && matter.status !== "closed" && (
          <button
            disabled={busy || mode !== null}
            onClick={() => open("archive")}
          >
            Archive matter
          </button>
        )}
      </div>
      {notice && (
        <p className="notice success" role="status">
          {notice}
        </p>
      )}
      {error && (
        <p className="notice error" role="alert">
          {error}
        </p>
      )}
      {mode && (
        <form
          className="matter-edit-form"
          onSubmit={(e) => {
            e.preventDefault();
            void submit();
          }}
        >
          <h3>
            {mode === "edit"
              ? "Edit matter"
              : mode === "delete"
                ? "Delete this draft?"
                : "Archive this matter?"}
          </h3>
          <fieldset disabled={busy}>
            {mode === "edit" ? (
              <>
                <label>
                  Matter title
                  <input
                    required
                    maxLength={200}
                    value={title}
                    onChange={(e) => setTitle(e.target.value)}
                  />
                </label>
                <label>
                  Fictional client label
                  <input
                    required
                    maxLength={120}
                    value={client}
                    onChange={(e) => setClient(e.target.value)}
                  />
                </label>
                <label>
                  Service category
                  <select
                    value={category}
                    onChange={(e) => setCategory(e.target.value)}
                  >
                    {[
                      "property",
                      "business",
                      "family",
                      "criminal",
                      "other",
                    ].map((value) => (
                      <option key={value}>{value}</option>
                    ))}
                  </select>
                </label>
                <label>
                  Assigned staff
                  <select
                    required
                    disabled={role !== "reviewer"}
                    value={assigned}
                    onChange={(e) => setAssigned(e.target.value)}
                  >
                    {staff.map((member) => (
                      <option key={member.id} value={member.id}>
                        {member.display_name}
                      </option>
                    ))}
                  </select>
                </label>
                <p className="muted small">
                  The reference and document checklist stay intact. Reassigning
                  a matter also moves its open follow-ups to the selected staff
                  member.
                </p>
              </>
            ) : (
              <>
                <p className="small">
                  {mode === "delete"
                    ? "This unused draft will be removed from the demo. Its reference will not be reused. This action cannot be undone in the demo."
                    : "This marks the matter Closed and prevents further workflow changes. Its checklist and history remain available. Historical analytics retain it, while active-work counts exclude it."}
                </p>
                <label>
                  Type {matter.reference} to confirm
                  <input
                    required
                    value={confirmed}
                    onChange={(e) => setConfirmed(e.target.value)}
                    autoComplete="off"
                  />
                </label>
              </>
            )}
            <div className="document-actions">
              <button
                className="primary"
                type="submit"
                disabled={
                  busy ||
                  (mode === "edit"
                    ? !staff.length
                    : confirmed.trim() !== matter.reference)
                }
              >
                {busy
                  ? "Saving..."
                  : mode === "edit"
                    ? "Save changes"
                    : mode === "delete"
                      ? "Delete draft"
                      : "Confirm archive"}
              </button>
              <button type="button" onClick={() => setMode(null)}>
                Cancel
              </button>
            </div>
          </fieldset>
        </form>
      )}
    </section>
  );
}
