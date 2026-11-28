import { useState } from "react";
type Props = {
  version: number;
  busy: boolean;
  onChange: (path: string, body: unknown, message: string) => Promise<boolean>;
};
export function NewRequirementForm({ version, busy, onChange }: Props) {
  const [open, setOpen] = useState(false);
  const [title, setTitle] = useState("");
  const [required, setRequired] = useState(true);
  async function submit() {
    if (
      await onChange(
        "/requirements",
        { expected_version: version, title, is_required: required },
        "Document added to the checklist. The ready check has been updated.",
      )
    ) {
      setOpen(false);
      setTitle("");
      setRequired(true);
    }
  }
  return (
    <div className="new-requirement">
      <button
        className="text-button"
        disabled={busy}
        onClick={() => setOpen((value) => !value)}
      >
        {open ? "Cancel new requirement" : "Add requirement"}
      </button>
      {open && (
        <form
          className="follow-up-form"
          onSubmit={(event) => {
            event.preventDefault();
            void submit();
          }}
        >
          <label>
            Document title
            <input
              required
              maxLength={200}
              disabled={busy}
              value={title}
              onChange={(event) => setTitle(event.target.value)}
            />
          </label>
          <label className="checkbox-label">
            <input
              type="checkbox"
              disabled={busy}
              checked={required}
              onChange={(event) => setRequired(event.target.checked)}
            />
            Needed before marking ready
          </label>
          <button type="submit" disabled={busy || !title.trim()}>
            Save requirement
          </button>
        </form>
      )}
    </div>
  );
}
