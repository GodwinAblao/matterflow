# Follow-ups and descriptive analytics

## Try it

Open a demo and select Analytics. Choose a service category, click a metric or bar to inspect its underlying records, and click a matter reference to open the checklist. Export CSV downloads the currently selected table, including its filters.

On a nonclosed matter, add a follow-up with a title and explicitly entered due date. Staff can update their own tasks on their assigned matters; reviewers can update any follow-up in the workspace. Tasks can be completed, reopened, or rescheduled. Reopen a completed task before rescheduling it. Due dates may be historical so the demo can represent existing overdue work. These are administrative dates, not automatically calculated legal deadlines.

New follow-ups default to the matter's assigned staff member. The demo limits each matter to fifty follow-ups. Every action checks the current session and role, locks the matter and task, validates its version, updates the task and matter version, and writes activity in one transaction. Closed matters reject changes. Follow-ups do not change document readiness or block its confirmation.

## Metric definitions

All report queries run in one repeatable-read, read-only PostgreSQL transaction. The response includes its snapshot instant and Philippines calendar date. The category filter applies to every dataset and aggregate. Staff sees assigned matters; staff task metrics further include only their assigned tasks. Reviewers see the current workspace. No other visitor's records enter a report.

| Metric                         | Definition and scope                                                                                                                                                    |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Visible matters                | All matters authorized for the user and selected category, including closed matters                                                                                     |
| Active matters                 | Visible matters whose stage is not closed; ready matters remain active                                                                                                  |
| Required document completeness | Accepted required documents divided by all required documents across visible matters, multiplied by 100; optional documents excluded, closed matters included           |
| Outstanding requirements       | Unaccepted required checklist items on nonclosed visible matters                                                                                                        |
| Requirement age                | Whole elapsed 24-hour periods since requested_at, clamped at zero; receiving a correction does not reset the original request time                                      |
| Overdue follow-ups             | Incomplete authorized follow-ups on nonclosed matters with due_on strictly before the snapshot's Asia/Manila date; tasks due today are not overdue                      |
| Median time to first readiness | Median elapsed days between created_at and first_ready_at, among visible matters with a first readiness timestamp; closed matters included, unfinished matters excluded |
| Open assignments               | Active matter counts grouped by staff assignee; a distribution measure rather than a productivity score                                                                 |

An empty document denominator or readiness cohort displays N/A. Percentages are rounded only for display; the API supplies the unrounded value. Even-size median cohorts use the mean of their two central observations. Outstanding requirement age uses buckets 0-7, 8-14, and 15+ days. Readiness metrics measure document preparation, not legal case outcomes.

## Drill-downs and CSV

The summary and record arrays come from the same snapshot. The browser derives stage, age, and assignment groups from those arrays, so each chart can be reconciled with its selected table. CSV exports the selected underlying records; it does not export an unrelated whole-database extract. Strings that could be interpreted as spreadsheet formulas are neutralized. Files are generated locally in the browser.

## API

- GET /api/v1/analytics, optionally with category=property, business, family, criminal, or other.
- POST /api/v1/matters/{id}/follow-ups with title and due_on (YYYY-MM-DD).
- PATCH /api/v1/matters/{matter_id}/follow-ups/{id} with expected_version, action, and due_on (required only for reschedule; null otherwise).

Task versions and can_edit are returned with matter details. Stale updates and invalid transitions return 409, invalid input returns 400, and ownership violations return 403. Follow-up actions are recorded as follow_up.created, completed, reopened, or rescheduled events.

## Verification

Run npm.cmd run test:analytics against the migrated and seeded development database. Tests create isolated demo workspaces and remove them on success. The controlled fixture verifies 50% completeness, a three-day readiness median, exclusion of unfinished matters from that cohort, empty denominators, role/workspace scope, due-today boundaries, task ownership, activity records, stale versions, and updated overdue counts after task actions. The pure Rust median test runs during ordinary cargo test without contacting Neon.
