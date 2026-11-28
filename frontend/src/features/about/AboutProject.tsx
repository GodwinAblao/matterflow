import { useRef } from "react";
export function AboutProject() {
  const dialog = useRef<HTMLDialogElement>(null);
  return (
    <>
      <button
        className="about-trigger"
        onClick={() => dialog.current?.showModal()}
      >
        About this project
      </button>
      <dialog
        className="about-dialog"
        ref={dialog}
        aria-labelledby="about-project-title"
      >
        <div className="about-dialog-heading">
          <div>
            <p className="eyebrow">PORTFOLIO PROJECT</p>
            <h2 id="about-project-title">Inside MatterFlow</h2>
          </div>
          <form method="dialog">
            <button autoFocus aria-label="Close project overview">
              Close
            </button>
          </form>
        </div>
        <p className="about-lead">
          A legal operations prototype that turns missing documents, review
          decisions, and follow-ups into a clear readiness workflow.
        </p>
        <section>
          <h3>Why this project exists</h3>
          <p>
            Document preparation can stall when requirements, correction
            requests, and responsibility are unclear. MatterFlow makes those
            blockers visible and records each step, then summarizes the
            operational data for staff and reviewers.
          </p>
        </section>
        <section>
          <h3>Built with Rust at the core</h3>
          <ol className="architecture-flow">
            <li>
              <strong>React + TypeScript</strong>
              <span>Forms, role views, charts and record drill-downs</span>
            </li>
            <li>
              <strong>Rust + Axum + SQLx</strong>
              <span>
                Permissions, workflow transitions, transactions and analytics
              </span>
            </li>
            <li>
              <strong>PostgreSQL on Neon</strong>
              <span>
                Versioned schema, activity records and isolated demo workspaces
              </span>
            </li>
          </ol>
        </section>
        <section>
          <h3>Engineering you can inspect</h3>
          <ul>
            <li>Staff and reviewer permissions are checked on the server.</li>
            <li>Row locks and version checks protect concurrent updates.</li>
            <li>A workflow change and its activity event commit together.</li>
            <li>
              Creation retries return the same matter rather than creating
              duplicates.
            </li>
            <li>
              Each visitor works in a separate, expiring fictional workspace.
            </li>
          </ul>
        </section>
        <section>
          <h3>Business Analytics contribution</h3>
          <p>
            Reports show approved documents, documents still waiting for
            approval, overdue follow-ups, typical preparation time, and staff
            assignments. Select a number or chart to see its records and
            download them as CSV.
          </p>
          <p>
            Readiness time measures document preparation. These figures do not
            predict legal outcomes or rate staff productivity.
          </p>
        </section>
        <section>
          <h3>Project ownership and scope</h3>
          <p>
            A portfolio project by Godwin Aldrich M. Ablao, BS Information
            Technology specializing in Business Analytics at FEU Institute of
            Technology. The project combines application design, Rust API
            development, database modeling, analytics definitions, and workflow
            validation.
          </p>
          <p>
            This is an independent internship portfolio prototype inspired by
            publicly described legal-office workflows. It is not an official
            Salenga Law Firm system. Every demo client, matter, and requirement
            is fictional. Production identity login and legal deadline
            calculation are outside its scope.
          </p>
        </section>
        <a
          className="source-link"
          href="https://github.com/GodwinAblao/matterflow"
          target="_blank"
          rel="noreferrer"
        >
          View source and technical documentation
        </a>
      </dialog>
    </>
  );
}
