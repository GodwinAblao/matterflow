# Architecture

## Boundaries

The browser calls the Rust API. Only the backend connects to PostgreSQL. React displays server responses and collects user input; Rust will enforce permissions, workflow transitions, and analytics definitions.

Backend features will use handler.rs for HTTP parsing, service.rs for business rules, repository.rs for SQL, and model.rs for feature types when those layers become necessary. Avoid creating empty layers before there is behavior to implement.

## Planned workflow

Create matter -> define required documents -> record receipt -> review -> resolve missing requirements -> confirm readiness.

Two planned roles: staff and lawyer/reviewer. Public demo role selection must create a server-authorized isolated demo session; trusting a browser role flag is insufficient. Authentication and authorization are not implemented in this foundation.

Status changes and their activity records should be committed in the same database transaction. Record receipt and review acceptance are separate events. Readiness checks must return explicit blockers. Dates are staff-entered administrative dates, not automatically calculated legal deadlines.

## Planned features

- matters: register, assignment, search, status
- documents: required checklists, receipt, acceptance, corrections
- follow_ups: ownership, due dates, outstanding work
- analytics: documented aggregations and drill-downs
- activity: history written with workflow changes
- auth: role enforcement and isolated demo sessions

Each feature folder currently contains a scope note only, except health. No business behavior is implied by an existing folder.

## Analytics definitions to implement

- Completeness: accepted required documents / all required documents; show N/A for an empty checklist.
- Outstanding requirement age: elapsed days since request for each unresolved requirement.
- Overdue follow-ups: incomplete follow-ups past their explicitly entered due date.
- Time to readiness: median elapsed time from matter opening to first readiness, among matters that reached readiness.
- Work distribution: active matter/task counts by assignee; counts do not measure staff productivity.

Use Asia/Manila for displayed administrative dates and UTC instants for event timestamps. Test boundary conditions and keep unfinished matters visible in aging reports.

## Next milestones

1. PostgreSQL schema and synthetic seed data.
2. Server-managed demo sessions and role permissions.
3. One complete matter/document workflow, including transactional history.
4. Follow-ups and descriptive analytics.
5. Deployment, reviewer walkthrough, and portfolio documentation.
