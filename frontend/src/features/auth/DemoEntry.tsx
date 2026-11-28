import { useState } from "react";
import type { Role } from "./useDemoSession";

type Props = {
  busy: boolean;
  ready: boolean;
  onEnter: (role: Role) => Promise<void>;
};

export function DemoEntry({ busy, ready, onEnter }: Props) {
  const [role, setRole] = useState<Role>("staff");
  return (
    <form
      className="demo-entry"
      onSubmit={(event) => {
        event.preventDefault();
        if (ready && !busy) void onEnter(role);
      }}
    >
      <fieldset disabled={busy}>
        <legend>Choose your demo role</legend>
        <div className="role-options">
          <label
            className={
              role === "staff" ? "role-option selected" : "role-option"
            }
          >
            <input
              type="radio"
              name="role"
              value="staff"
              checked={role === "staff"}
              onChange={() => setRole("staff")}
            />
            <span>
              <strong>Paralegal / Staff</strong>
              <small>Explore assigned matters and document checklists.</small>
            </span>
          </label>
          <label
            className={
              role === "reviewer" ? "role-option selected" : "role-option"
            }
          >
            <input
              type="radio"
              name="role"
              value="reviewer"
              checked={role === "reviewer"}
              onChange={() => setRole("reviewer")}
            />
            <span>
              <strong>Lawyer / Reviewer</strong>
              <small>Explore all matters and the team workload.</small>
            </span>
          </label>
        </div>
      </fieldset>
      <button className="primary" disabled={busy || !ready} type="submit">
        {busy
          ? "Preparing your workspace..."
          : ready
            ? "Explore Demo"
            : "Getting demo ready..."}
      </button>
      <p className="muted small">
        No registration needed. Your private sample workspace lasts two hours.
        You can switch roles inside.
      </p>
    </form>
  );
}
