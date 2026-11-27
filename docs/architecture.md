# Architecture

## Boundaries

The browser calls the Rust API. Only the backend connects to PostgreSQL. React displays server responses and collects user input; Rust will enforce permissions, workflow transitions, and analytics definitions.

Backend features will use handler.rs for HTTP parsing, service.rs for business rules, repository.rs for SQL, and model.rs for feature types when those layers become necessary. Avoid creating empty layers before there is behavior to implement.

## Planned workflow

Create matter -> define required documents -> record receipt -> review -> resolve missing requirements -> confirm readiness.

Two demo roles are implemented: staff and reviewer. Public role selection creates a server-authorized isolated demo session. Each request reads the active role and workspace from PostgreSQL; browser role flags do not grant access. Production identity login is outside the current demo scope.

Status changes and their activity records should be committed in the same database transaction. Record receipt and review acceptance are separate events. Readiness checks must return explicit blockers. Dates are staff-entered administrative dates, not automatically calculated legal deadlines.

## Planned features

- matters: register, assignment, search, status
- documents: required checklists, receipt, acceptance, corrections
- follow_ups: ownership, due dates, outstanding work
- analytics: documented aggregations and drill-downs
- activity: history written with workflow changes
- auth: role enforcement and isolated demo sessions

Health, session lifecycle, matter register/detail reads, document receipt/review transitions, and readiness confirmation are implemented. Other feature folders contain scope notes for later work. The auth service owns transactional demo creation; handlers manage HTTP input and output.

## Analytics definitions to implement

- Completeness: accepted required documents / all required documents; show N/A for an empty checklist.
- Outstanding requirement age: elapsed days since request for each unresolved requirement.
- Overdue follow-ups: incomplete follow-ups past their explicitly entered due date.
- Time to readiness: median elapsed time from matter opening to first readiness, among matters that reached readiness.
- Work distribution: active matter/task counts by assignee; counts do not measure staff productivity.

Use Asia/Manila for displayed administrative dates and UTC instants for event timestamps. Test boundary conditions and keep unfinished matters visible in aging reports.

## Next milestones

1. Add follow-up actions and descriptive analytics with drill-downs.
2. Deploy, verify the hosted demo, and prepare the reviewer walkthrough and portfolio documentation.
