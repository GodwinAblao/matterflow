# Matter CRUD

MatterFlow supports creation, reading, editing, and logical deletion of unused drafts. Reviewers can also archive matters with workflow history.

## Try it

1. Create a fictional matter using New matter.
2. Select it and choose Edit matter, correct the title, and save. The reference and checklist are preserved.
3. Edit the client label or category. Reviewers can reassign staff; open follow-ups move with the assignment.
4. For a new intake draft without workflow activity, choose Delete draft and type its exact reference. Cancel leaves it unchanged; confirming removes it from the register and reports.
5. Once receipt/review, a checklist addition, or a follow-up has been recorded, deletion is blocked. A reviewer can instead choose Archive matter and confirm its reference. The matter becomes Closed, its records remain readable, and further workflow changes are blocked.

Staff may edit or delete eligible assigned matters. Reviewers may manage any matter in their workspace. Ready and closed matters cannot be edited. Archiving is reviewer-only, and this version does not offer reopening.

## Data rules

- All mutations authorize the current actor, lock the matter, and check expected_version before writing.
- Editing records the before/after metadata in the activity log. Archiving and deletion also create events in the same transaction.
- Deletion is logical: deleted_at hides the draft from the register, details, team counts, analytics and workflow mutation endpoints. Its database rows and audit trail remain until workspace cleanup.
- A deleted reference is never reused. Its creation key cannot recreate or replay the deleted draft. The fifty-matter workspace limit includes deleted drafts.
- Deletion eligibility requires a user-created intake matter, untouched missing checklist items, no follow-ups, and no activity except creation or metadata edits. Seeded matters cannot be deleted.
- Archived matters use the existing Closed stage. They remain in historical document completeness and readiness cohorts but are excluded from active workload, outstanding requirement and overdue-task metrics. Archiving is an administrative action, not a declaration of legal resolution.
- An expired workspace is still removed through the normal demo lifecycle.

## API

- PATCH /api/v1/matters/{id}: expected_version, title, client_label, service_category and assigned_staff_id.
- DELETE /api/v1/matters/{id}: expected_version and reference, returning 204 on success.
- POST /api/v1/matters/{id}/archive: expected_version and reference.

Invalid or stale operations do not partially change the matter. Conflicts return 409; role violations return 403; inaccessible or deleted records return 404.

## Verification and deployment

Run npm.cmd run test:management against the migrated and seeded database. The integration tests cover title correction, stale writes, input validation, workspace isolation, assignment permissions, deletion visibility, retained audit history, creation-key handling, nonreused references, task reassignment and archived workflow locks. Frontend tests cover editing, deletion confirmation, cancellation and role controls.

Migration 202611280006 adds deleted_at and a visibility index. Apply migrations before deploying the backend, then deploy the frontend. The additive migration was applied to the configured Neon database during implementation; both application services still require deployment of this change.
