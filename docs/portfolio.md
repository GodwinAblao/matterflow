# MatterFlow — project submission

Prepared by Godwin Aldrich M. Ablao for the Programmer OJT - Rust student internship application at Salenga Law Firm.

- Live demo: https://godwinablao-matterflow.onrender.com
- Source code: https://github.com/GodwinAblao/matterflow
- Project walkthrough: this document

## Purpose

MatterFlow is an independent legal document readiness and operations analytics prototype. It demonstrates how a legal office could track outstanding checklist items, review corrections, coordinate administrative follow-ups, and identify preparation bottlenecks.

The proposed workflow is a portfolio concept, not a representation of Salenga Law Firm's internal processes or an official firm system. All sample clients, matters, and checklist requirements are fictional. The application records checklist status; it does not upload or store document files, provide legal advice, or calculate statutory deadlines.

## Technical contribution

Rust is the backend implementation language, with Axum for HTTP routing, Tokio for asynchronous execution, and SQLx for PostgreSQL access. PostgreSQL is hosted on Neon. The React and TypeScript frontend is built with Vite. The frontend and Rust service are deployed on Render.

The project demonstrates:

- Backend authorization: staff accesses assigned matters; reviewers access the workspace's matters and team workload. These rules are checked by the Rust API.
- Matter management: create and edit metadata, reassign staff as reviewer, delete unused drafts with reference confirmation, and archive matters while preserving history.
- Workflow integrity: document receipt, correction requests, resubmission, acceptance, and readiness confirmation have explicit permissions and blockers.
- Database consistency: transactions, row locks, version checks, and atomic activity history protect concurrent updates. Creation keys prevent duplicate matters on retries.
- Data isolation: each visitor receives a separate, expiring demo workspace. Role switching is deliberately available for evaluation and is not production authentication.
- Descriptive analytics: Rust and SQL produce scoped report data; TypeScript renders charts, record drill-downs, and CSV downloads.
- Maintainability: feature modules, versioned migrations, repeatable fictional seed data, documented metric definitions, and automated checks.

This submission focuses on Rust rather than claiming C++, Node.js backend, or ASP.NET experience from this project. Node.js supplies the frontend development/build tools.

## Five-minute walkthrough

1. Open the demo and choose Explore Demo. Start as Staff. The initial register shows six assigned matters. Use the search and pagination controls.
2. Create a fictional matter in the property category. Add one required checklist item and optionally a supporting item. The new matter appears in the register.
3. Choose Edit matter to correct its title while preserving its reference. Record receipt of its required item: the Received count increases, while the accepted count changes only after reviewer acceptance.
4. Switch to Reviewer. Request a correction with a note. Observe that document readiness remains blocked.
5. Switch to Staff, record resubmission, then switch back to Reviewer and accept the item. Confirm readiness once all required items are accepted. Review the activity history.
6. Add an administrative follow-up with an explicit date. Inspect Analytics, filter by category, and drill into the relevant records. Complete the follow-up and observe the report change. Export a filtered table as CSV.

Reviewers initially see twelve seeded matters, plus any matters created in their workspace. To evaluate deletion, create an extra unused draft, choose Delete draft, and confirm its reference. Reviewers can archive matters with workflow history; they become Closed and retain their records. A new visitor's workspace starts separately. Sessions expire after two hours; do not enter real client information.

## Analytics and interpretation

| Measure                        | Meaning                                                                                                                   |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------- |
| Required document completeness | Accepted required checklist items divided by all required items in the authorized, filtered matter set                    |
| Outstanding requirements       | Unaccepted required items on nonclosed matters                                                                            |
| Requirement age                | Whole elapsed days since the checklist item was requested                                                                 |
| Overdue follow-ups             | Incomplete tasks on nonclosed matters due before the Philippines calendar date; tasks due today are excluded              |
| Median time to first readiness | Median days from matter creation to its first readiness confirmation, excluding matters that have never reached readiness |
| Open assignments               | Active matter counts by assignee, shown to reviewers                                                                      |

These measures describe document preparation and administrative workload. They do not estimate legal outcomes or rank staff productivity. Empty denominators and empty readiness cohorts display N/A. See [analytics.md](analytics.md) for exact scope and calculation rules.

## Verification and limits

On November 28, 2026 (Asia/Singapore), the deployed service passed 37 HTTP-level checks covering readiness, matter creation and safe retries, staff/reviewer scope, document corrections and resubmissions, readiness blockers, follow-up completion, analytics updates, activity history, frontend availability, and CORS. The temporary verification workspace was removed afterward.

These checks verify deployed API behavior and frontend delivery, not a complete browser or accessibility review. Register pagination, CSV download, mobile layout, and visual interaction should also be checked manually before submission.

Free hosting may require a wait after inactivity. A database readiness request briefly timed out during deployment before a retry succeeded. The prototype currently has public demo role switching, session and workspace limits, and manual cleanup tooling. Real office use would require production identity management, operational monitoring, backups and retention policies, and a privacy/security review appropriate to actual client records.

## Supporting documentation

- [Architecture](architecture.md)
- [Database](database.md)
- [Demo lifecycle](demo.md)
- [Document workflow](workflow.md)
- [Matter intake](intake.md)
- [Matter CRUD](crud.md)
- [Two-minute recording script](demo-recording.md)
- [Analytics](analytics.md)
- [Deployment](deployment.md)
