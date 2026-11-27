# Document review and readiness

## Try the complete workflow

1. Open http://localhost:5173 and enter as Staff.
2. Select MF-001. It starts at intake with missing requirements.
3. Use Record receipt on its three required checklist items. Leave the optional item missing to see that it does not block readiness.
4. Switch to Reviewer; MF-001 remains selected. Accept two required documents and request a correction on the third, explaining what is needed.
5. Readiness lists the outstanding correction. A received document is not treated as accepted.
6. Switch to Staff and record the corrected receipt on the same matter. The document returns to Received and its previous correction remains in activity history.
7. Switch to Reviewer and accept the corrected document.
8. Mark matter ready. The register, counts, readiness result, and activity history refresh.

The selected matter is retained across role switches when the new role can access it. A role-specific list is fetched before rendering any detail; inaccessible selections are hidden. A new demo workspace resets selection. All records and requirements are fictional. Record receipt stores the time of the action; it does not upload a file.

## Rules

| Action                   | Role           | Allowed starting document state                                                            |
| ------------------------ | -------------- | ------------------------------------------------------------------------------------------ |
| Record receipt           | Assigned staff | Missing                                                                                    |
| Record corrected receipt | Assigned staff | Needs correction                                                                           |
| Accept document          | Reviewer       | Received                                                                                   |
| Request correction       | Reviewer       | Received; nonempty explanation required                                                    |
| Mark matter ready        | Reviewer       | Nonclosed matter, with at least one required document and every required document accepted |

A correction explanation is trimmed and limited to 1,000 characters. Acceptance cannot include a correction explanation. Ready and closed matters reject checklist changes. Reopening/closure actions are outside this milestone.

A receipt or review updates the administrative stage to Collecting documents while required items are missing or need correction; otherwise the stage is Under review. Completing the checklist never automatically marks a matter ready. Optional documents do not block readiness.

## API

All routes are under /api/v1 and require the demo bearer token.

- POST /matters/{matter_id}/documents/{id}/receive with expected_version from the document.
- POST /matters/{matter_id}/documents/{id}/review with expected_version, outcome (accepted or needs_correction), and note (null for acceptance).
- POST /matters/{id}/ready with expected_version from the matter.

GET /matters/{id} now includes document versions and timestamps, readiness blockers, and the latest 100 activity entries. Details are read from one repeatable-read snapshot so the checklist and readiness result agree.

Invalid transitions and stale versions return 409. Incomplete readiness returns error.code=readiness_blocked and error.details.blockers containing titles, statuses, and explanations. Invalid explanations return 400; malformed/unknown request fields are rejected by JSON validation. Role violations return 403; inaccessible matter/document IDs return 404.

Repeating a successful readiness confirmation is idempotent: it does not create another event or replace first_ready_at.

## Transactions and concurrency

Each write opens a transaction, locks/revalidates the current workspace and session, checks the current database role, then locks the matter before the document. Receipt, review, and readiness all follow this order. Role changes and demo termination wait for an in-flight mutation's authorization locks.

Document writes require the caller's document version. Each successful document write increments document and matter versions. Follow-up changes increment their own version and the matter version as well. Readiness confirmation requires the displayed matter version, preventing confirmation based on an out-of-date checklist. A concurrent final review requires the readiness caller to refresh before confirming.

Record changes and activity inserts commit together. Rejected transitions and stale requests produce no workflow event. The correction explanation is copied into the event so later resubmission does not erase history. first_ready_at is set only if absent. Event timestamps use the database clock after locks are acquired, avoiding transaction-start times that could predate a concurrent receipt.

Current activity is protected by the API's available routes, not an append-only database role. A privileged database owner can still edit rows directly. This demo is administrative workflow software and does not calculate legal deadlines.

## Verification

Run npm.cmd run test:workflow against the migrated and seeded development database. Tests create their own isolated demo workspaces and remove them on success; failed workspaces expire normally. They cover permissions, cross-workspace access, correction validation/resubmission, retained explanations, optional requirements, stale versions, repeated readiness, rejected writes producing no activity, simultaneous receipts, a last-review/readiness race, and an empty required checklist.
