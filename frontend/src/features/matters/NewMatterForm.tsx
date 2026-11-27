import { useEffect, useState } from "react";
import { ApiError, apiGet, apiRequest } from "../../lib/api";
import type { Role } from "../auth/useDemoSession";
type Staff = { id: string; display_name: string };
type Row = { key: string; title: string; is_required: boolean };
type Props = {
  token: string;
  role: Role;
  onExpired: () => void;
  onCreated: (id: string, reference: string) => void;
  onCancel: () => void;
};
export function NewMatterForm({
  token,
  role,
  onExpired,
  onCreated,
  onCancel,
}: Props) {
  const [title, setTitle] = useState("");
  const [client, setClient] = useState("");
  const [category, setCategory] = useState("property");
  const [assigned, setAssigned] = useState("");
  const [rows, setRows] = useState<Row[]>(() => [
    { key: crypto.randomUUID(), title: "", is_required: true },
  ]);
  const [creationKey, setCreationKey] = useState(() => crypto.randomUUID());
  const [staff, setStaff] = useState<Staff[] | null>(null);
  const [loadError, setLoadError] = useState("");
  const [attempt, setAttempt] = useState(0);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    const controller = new AbortController();
    void apiGet<Staff[]>("/matters/options", controller.signal, token)
      .then((options) => {
        if (!controller.signal.aborted) {
          setStaff(options);
          setLoadError(
            options.length ? "" : "No staff member is available in this demo.",
          );
        }
      })
      .catch((failure: unknown) => {
        if (controller.signal.aborted) return;
        if (failure instanceof ApiError && failure.status === 401) onExpired();
        else setLoadError("Could not load staff assignments. Please retry.");
      });
    return () => controller.abort();
  }, [token, onExpired, attempt]);
  function edit(change: () => void) {
    change();
    setCreationKey(crypto.randomUUID());
    setError("");
  }
  function updateRow(key: string, changes: Partial<Row>) {
    edit(() =>
      setRows((current) =>
        current.map((row) => (row.key === key ? { ...row, ...changes } : row)),
      ),
    );
  }
  async function submit() {
    setError("");
    if (!rows.some((row) => row.is_required)) {
      setError("Mark at least one document as required.");
      return;
    }
    if (!staff?.length) return;
    setBusy(true);
    try {
      const created = await apiRequest<{ id: string; reference: string }>(
        "/matters",
        {
          method: "POST",
          token,
          body: {
            creation_key: creationKey,
            title,
            client_label: client,
            service_category: category,
            assigned_staff_id: assigned || staff[0].id,
            documents: rows.map(({ title, is_required }) => ({
              title,
              is_required,
            })),
          },
        },
      );
      onCreated(created.id, created.reference);
    } catch (failure) {
      if (failure instanceof ApiError && failure.status === 401) onExpired();
      else
        setError(
          failure instanceof Error
            ? failure.message
            : "Could not create the matter.",
        );
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="panel intake-panel" aria-label="New matter">
      <div className="panel-heading">
        <div>
          <h2>New fictional matter</h2>
          <p className="muted small">
            Set up the work and its document requirements. Use fictional client
            details.
          </p>
        </div>
        <button disabled={busy} onClick={onCancel}>
          Cancel
        </button>
      </div>
      {loadError && (
        <p className="notice error workflow-notice" role="alert">
          {loadError}
          <button onClick={() => setAttempt((value) => value + 1)}>
            Retry assignments
          </button>
        </p>
      )}
      {error && (
        <p className="notice error workflow-notice" role="alert">
          {error}
        </p>
      )}
      <form
        className="intake-form"
        onSubmit={(event) => {
          event.preventDefault();
          void submit();
        }}
      >
        <fieldset disabled={busy || !staff?.length}>
          <div className="intake-fields">
            <label>
              Matter title
              <input
                required
                maxLength={200}
                value={title}
                onChange={(event) => edit(() => setTitle(event.target.value))}
                placeholder="e.g. Demo property document review"
              />
            </label>
            <label>
              Fictional client label
              <input
                required
                maxLength={120}
                value={client}
                onChange={(event) => edit(() => setClient(event.target.value))}
                placeholder="e.g. Demo Client 13"
              />
            </label>
            <label>
              Service category
              <select
                value={category}
                onChange={(event) =>
                  edit(() => setCategory(event.target.value))
                }
              >
                {["property", "business", "family", "criminal", "other"].map(
                  (value) => (
                    <option key={value} value={value}>
                      {value}
                    </option>
                  ),
                )}
              </select>
            </label>
            <label>
              Assigned staff
              {role === "reviewer" ? (
                <select
                  value={assigned || staff?.[0]?.id || ""}
                  onChange={(event) =>
                    edit(() => setAssigned(event.target.value))
                  }
                >
                  {staff?.map((member) => (
                    <option key={member.id} value={member.id}>
                      {member.display_name}
                    </option>
                  ))}
                </select>
              ) : (
                <input
                  value={staff?.[0]?.display_name ?? "Loading..."}
                  readOnly
                />
              )}
            </label>
          </div>
          <div className="checklist-builder-heading">
            <div>
              <h3>Initial document checklist</h3>
              <p className="muted small">
                Define your own requirements. Include at least one required
                document.
              </p>
            </div>
            <button
              type="button"
              disabled={rows.length >= 20}
              onClick={() =>
                edit(() =>
                  setRows((current) => [
                    ...current,
                    { key: crypto.randomUUID(), title: "", is_required: true },
                  ]),
                )
              }
            >
              Add checklist item
            </button>
          </div>
          <div className="checklist-builder">
            {rows.map((row, index) => (
              <div className="checklist-builder-row" key={row.key}>
                <label>
                  <span className="sr-only">Document {index + 1} title</span>
                  <input
                    required
                    maxLength={200}
                    value={row.title}
                    placeholder={"Document " + (index + 1) + " title"}
                    onChange={(event) =>
                      updateRow(row.key, { title: event.target.value })
                    }
                  />
                </label>
                <label className="checkbox-label">
                  <input
                    type="checkbox"
                    checked={row.is_required}
                    onChange={(event) =>
                      updateRow(row.key, { is_required: event.target.checked })
                    }
                  />
                  Required
                </label>
                <button
                  type="button"
                  disabled={rows.length === 1}
                  aria-label={"Remove document " + (index + 1)}
                  onClick={() =>
                    edit(() =>
                      setRows((current) =>
                        current.filter((item) => item.key !== row.key),
                      ),
                    )
                  }
                >
                  Remove
                </button>
              </div>
            ))}
          </div>
          <div className="intake-submit">
            <p className="muted small">
              A matter reference is generated when saved. All checklist items
              start as missing.
            </p>
            <button
              className="primary"
              type="submit"
              disabled={busy || !staff?.length}
            >
              {busy ? "Creating matter..." : "Create matter"}
            </button>
          </div>
        </fieldset>
      </form>
    </section>
  );
}
