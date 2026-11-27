# MatterFlow database

## Connection and commands

DATABASE_URL belongs in backend/.env or the backend hosting service's secret environment variables. Preserve the Neon connection string's SSL options. Never put database credentials into frontend variables or Git.

Run all commands from the repository root:

```powershell
npm.cmd run db:check
npm.cmd run db:migrate
npm.cmd run db:test
```

The check command verifies connectivity and reports the number of MatterFlow tables without displaying credentials. Migrations require schema/table creation permissions. The migration command applies pending migrations and verifies prior checksums; it is safe to rerun. It does not reset or delete records. Back up data before future migrations that alter populated tables.

The opt-in database test uses this development database and rolls back all fixture rows. Ordinary cargo test does not contact Neon.

## Tables

| Table in matterflow schema | Purpose                                                                             |
| -------------------------- | ----------------------------------------------------------------------------------- |
| workspaces                 | Isolated demo data groups, with optional expiry and template designation            |
| app_users                  | Staff/reviewer identities belonging to one workspace                                |
| demo_sessions              | Expiring session records; store hashed tokens only                                  |
| matters                    | References, service category, assignments, stage, first readiness and closure times |
| document_requirements      | Required/optional checklists, receipt, review outcome, reviewer and correction note |
| follow_ups                 | Matter/document follow-ups with assignee, local due date and completion time        |
| activity_events            | Timestamped matter activity and structured change details                           |

Composite foreign keys prevent references across workspaces, including reviewers, assignees, and activity actors. Document links on follow-ups also include the matter ID so a follow-up cannot reference another matter's requirement.

A document cannot be accepted without receipt/review times and a reviewer. A correction requires a nonempty explanation. Matter references are unique within each workspace. First-readiness timestamps cannot precede creation. These facts support later operational analytics.

## Responsibilities still to implement

The Rust session extractor validates the token, current database role, workspace, and expiry on each protected request. Matter reads are scoped by workspace; staff is additionally restricted to assigned matters. Reviewer-only team access is enforced in the handler. Database integrity constraints supplement these application checks; they do not replace them. Workflow transition rules remain to be implemented.

The readiness service must lock the matter while validating current checklist items and changing stage, and checklist mutations must follow the same locking convention. The database cannot determine checklist completeness with a row-level CHECK. Stage changes and activity inserts must use one transaction. Do not expose mutation routes until these rules exist.

Activity is not yet append-only at the database permission level. Introduce a restricted runtime database role before treating the activity log as protected against direct database edits. No public database credentials are used in the frontend.

Current review columns describe the latest review; record prior reviews and corrections in activity details when implementing the workflow. Preserve first_ready_at after reopening for consistent time-to-first-readiness analytics. Set updated_at explicitly on matter updates.

Administrative follow-up dates use Asia/Manila. Store event instants as TIMESTAMPTZ. No legal deadline calculations are performed. No client documents are stored in this schema; document records are checklist metadata.

## Data and next step

Migrations create schema only. The explicit seed command creates a fictional template with two staff users and one reviewer. Demo entry clones the template into an expiring workspace per visitor. See demo.md for dataset counts and lifecycle behavior.

## Demo lifecycle implemented

The second migration adds a unique template key and one session per workspace. The seed and session clone lifecycle are now implemented; see demo.md. API requests are scoped to a validated workspace, and staff/reviewer read permissions are enforced. Document workflow mutations remain unimplemented.
