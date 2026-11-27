import { useState } from "react";
export type FollowUp = {
  id: string;
  title: string;
  due_on: string;
  completed_at: string | null;
  version: number;
  assigned_name: string | null;
  can_edit: boolean;
};
type Props = {
  rows: FollowUp[];
  closed: boolean;
  busy: boolean;
  onChange: (
    path: string,
    body: unknown,
    message: string,
    method?: "POST" | "PATCH",
  ) => Promise<boolean>;
};
function today() {
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone: "Asia/Manila",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(new Date());
  return ["year", "month", "day"]
    .map((key) => parts.find((part) => part.type === key)?.value)
    .join("-");
}
export function FollowUpPanel({ rows, closed, busy, onChange }: Props) {
  const [adding, setAdding] = useState(false);
  const [title, setTitle] = useState("");
  const [due, setDue] = useState(today);
  const [editing, setEditing] = useState<string | null>(null);
  const [newDue, setNewDue] = useState("");
  async function create() {
    if (
      await onChange(
        "/follow-ups",
        { title, due_on: due },
        "Follow-up created and assigned to the matter staff.",
      )
    ) {
      setAdding(false);
      setTitle("");
    }
  }
  async function reschedule(row: FollowUp) {
    if (
      await onChange(
        "/follow-ups/" + row.id,
        { expected_version: row.version, action: "reschedule", due_on: newDue },
        "Follow-up rescheduled.",
        "PATCH",
      )
    )
      setEditing(null);
  }
  return (
    <section className="follow-up-panel">
      <div className="follow-up-heading">
        <h3>Follow-ups</h3>
        {!closed && (
          <button
            className="text-button"
            disabled={busy}
            onClick={() => setAdding((value) => !value)}
          >
            {adding ? "Cancel new follow-up" : "Add follow-up"}
          </button>
        )}
      </div>
      {adding && !closed && (
        <form
          className="follow-up-form"
          onSubmit={(event) => {
            event.preventDefault();
            void create();
          }}
        >
          <label>
            Task
            <input
              required
              maxLength={200}
              value={title}
              disabled={busy}
              onChange={(event) => setTitle(event.target.value)}
            />
          </label>
          <label>
            Due date
            <input
              type="date"
              required
              value={due}
              disabled={busy}
              onChange={(event) => setDue(event.target.value)}
            />
          </label>
          <button disabled={busy || !title.trim()} type="submit">
            Create follow-up
          </button>
        </form>
      )}
      <ul className="follow-ups">
        {rows.map((row) => (
          <li key={row.id}>
            <strong>{row.title}</strong>
            <p className="muted small">
              Due {row.due_on} -{" "}
              {row.completed_at ? "Completed" : "Outstanding"}
              <br />
              {row.assigned_name ?? "Unassigned"}
            </p>
            {row.can_edit && !closed && (
              <>
                <div className="document-actions">
                  <button
                    disabled={busy}
                    onClick={() =>
                      void onChange(
                        "/follow-ups/" + row.id,
                        {
                          expected_version: row.version,
                          action: row.completed_at ? "reopen" : "complete",
                          due_on: null,
                        },
                        row.completed_at
                          ? "Follow-up reopened."
                          : "Follow-up completed.",
                        "PATCH",
                      )
                    }
                  >
                    {row.completed_at ? "Reopen" : "Mark complete"}
                  </button>
                  {!row.completed_at && (
                    <button
                      disabled={busy}
                      onClick={() => {
                        setEditing(row.id);
                        setNewDue(row.due_on);
                      }}
                    >
                      Reschedule
                    </button>
                  )}
                </div>
                {editing === row.id && !row.completed_at && (
                  <form
                    className="follow-up-form"
                    onSubmit={(event) => {
                      event.preventDefault();
                      void reschedule(row);
                    }}
                  >
                    <label>
                      New due date
                      <input
                        type="date"
                        required
                        value={newDue}
                        disabled={busy}
                        onChange={(event) => setNewDue(event.target.value)}
                      />
                    </label>
                    <div className="document-actions">
                      <button
                        type="submit"
                        disabled={busy || newDue === row.due_on}
                      >
                        Save date
                      </button>
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => setEditing(null)}
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
      {rows.length === 0 && (
        <p className="muted small workflow-notice">No follow-ups yet.</p>
      )}
    </section>
  );
}
