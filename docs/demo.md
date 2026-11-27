# Fictional demo sessions

## Setup

From the repository root, after setting backend/.env:

```powershell
npm.cmd run db:migrate
npm.cmd run db:seed
npm.cmd run dev:api
```

In another terminal run npm.cmd run dev, then open http://localhost:5173. Select Staff or Reviewer and choose Explore Demo. Staff sees six assigned matters; the reviewer sees all twelve and the team workload. Selecting a matter opens its checklist and follow-ups.

Role switching is an explicit capability of this public fictional demo. It selects one of the current workspace's seeded identities on the server. It is not a production login system. There are two staff identities and one reviewer; the staff demo enters as Alex. There is no real client data or legal advice.

## Dataset

The idempotent seed creates one private portfolio-v1 template with three fictional users, twelve matters, forty-eight checklist items, twelve follow-ups, and opening activity. Existing template data is preserved on subsequent seed runs. A new template version should use a new key, not silently overwrite examples.

Each visit clones the template into a fresh random workspace, remaps all relationships, and shifts timestamps and Manila due dates relative to that visit. No visitor can read the template through an application endpoint.

## Session and permission rules

- A 256-bit operating-system random bearer token is returned only on demo creation. Only its SHA-256 digest is stored in PostgreSQL.
- The browser keeps the token in sessionStorage for refresh within the same tab. Tokens are not placed in URLs. Responses use Cache-Control: no-store.
- Every protected request looks up the session, current database role, workspace, and expiry. Client query parameters cannot change scope.
- Staff list/detail access is restricted to assigned matters; reviewer list/detail access covers the workspace. Team overview requires reviewer role.
- Switching roles changes the active user within the same isolated workspace. Unknown roles and extra role-selection fields are rejected.
- Sessions and workspaces expire after two hours. End demo deletes only that visitor's workspace and its dependent records.
- Creation is serialized with a PostgreSQL advisory lock and limited to thirty unexpired workspaces. Expired workspaces are cleaned up during creation. Run npm.cmd run db:cleanup to remove expired workspaces manually. The cap is a resource bound, not a per-client abuse prevention service.
- Before public deployment, keep HTTPS and an exact FRONTEND_ORIGIN. Add an ingress request-rate policy if abuse handling is needed. No credentials are included in the frontend.

## API

| Method | Path under /api/v1 | Purpose                                                 |
| ------ | ------------------ | ------------------------------------------------------- |
| POST   | /demo/sessions     | Create an isolated demo; JSON role is staff or reviewer |
| GET    | /demo/session      | Restore active identity and expiry                      |
| PATCH  | /demo/session      | Switch role within this demo                            |
| DELETE | /demo/session      | End this demo and remove its workspace                  |
| GET    | /matters           | List matters visible to the active role                 |
| GET    | /matters/{id}      | View an authorized matter, checklist and follow-ups     |
| GET    | /team              | Reviewer-only team workload                             |

Protected routes require Authorization: Bearer TOKEN. Invalid/expired sessions return 401, forbidden role access returns 403, and inaccessible matters return 404.

## Verification

Run npm.cmd run test:demo explicitly against the seeded development database. It creates two temporary demo workspaces and deletes them on success; failed test workspaces expire normally. It checks isolation, staff assignment filtering, reviewer permissions, role switching, session expiry, revocation, cloning, and seed repeatability. Ordinary cargo test stays offline from Neon.

Document receipt/review mutations, readiness transitions, and transactional activity recording are implemented; see workflow.md. Follow-up actions and the descriptive analytics dashboard are implemented; see analytics.md. Database-level append-only activity permissions remain a later hardening task.

New matters and initial document checklists can now be created from the Matters tab. See intake.md for the complete intake walkthrough and role behavior.

The Matter register displays five records per page by default, with 10/20-row options. Search applies to all authorized loaded matters and resets to the first page. Details remain open while navigating pages. Role switches reopen the page containing the selected matter when it remains authorized. Pagination is client-side over the bounded demo dataset (at most fifty matters per workspace).
