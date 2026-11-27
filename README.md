# MatterFlow

Legal document readiness and operations analytics portfolio prototype, built with Rust, Axum, SQLx, PostgreSQL, React, TypeScript, and Vite.

Independent project inspired by publicly described legal-office workflows. It is not an official Salenga Law Firm system. All sample clients, matters, and document requirements are fictional.

## What works

- Isolated two-hour demo workspaces, each with a copy of twelve sample matters.
- Staff/reviewer role switching, enforced by the Rust API on every protected request.
- Staff sees six assigned matters; reviewers see all twelve and the team workload.
- Searchable matter register, required-document progress, checklist details, and follow-ups.
- Versioned PostgreSQL migrations, repeatable synthetic seed, hashed session tokens, expiry, and cleanup.

Staff receipt/resubmission, reviewer acceptance/correction requests, readiness blockers, and transactional activity history are implemented. Follow-up creation/completion/reopening/rescheduling and the descriptive analytics dashboard are implemented, including category filters, interactive record drill-downs, and CSV export.

## Requirements

- Node.js 24 LTS and npm
- Stable Rust with Cargo, rustfmt, and Clippy (https://rustup.rs)
- On Windows: Visual Studio C++ Build Tools and a Windows SDK
- PostgreSQL or a Neon database connection string

## Local setup

From the repository root:

```powershell
npm.cmd install
if (-not (Test-Path backend/.env)) { Copy-Item backend/.env.example backend/.env }
```

Set DATABASE_URL in backend/.env, preserving the connection string's SSL settings. Never commit that file. Then:

```powershell
npm.cmd run db:migrate
npm.cmd run db:seed
npm.cmd run dev:api
```

In a second terminal:

```powershell
npm.cmd run dev
```

Open http://localhost:5173 and choose Explore Demo. Vite proxies /api requests to the Rust backend on port 3000. Use npm.cmd on Windows to avoid PowerShell execution-policy errors; npm works normally in other shells. Restart the Rust process after source or backend environment changes; Vite reloads frontend changes automatically.

## Structure

```text
matterflow/
  backend/
    src/
      app.rs               # Router and middleware
      config.rs            # Environment configuration
      state.rs             # Shared PostgreSQL pool
      error.rs             # JSON error responses
      bin/matterflow-db.rs # Explicit database commands
      db/                  # Pool and embedded migrations
      features/
        auth/              # Session extraction, HTTP handlers and demo service
        health.rs          # Liveness and database readiness
        matters/           # Scoped register and detail queries
    migrations/            # Versioned PostgreSQL migrations
    seeds/                 # Fictional template data
    tests/                 # Offline and opt-in database/API checks
  frontend/src/
    components/            # Shared UI
    features/              # Entry, sessions, matters and health status
    lib/                   # HTTP client
  docs/                    # Architecture, schema and demo behavior
```

## Verification

```powershell
npm.cmd run lint
npm.cmd run build
npm.cmd run format:check
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Live database checks are opt-in:

```powershell
npm.cmd run db:check
npm.cmd run db:test
npm.cmd run test:demo
npm.cmd run test:workflow
npm.cmd run test:analytics
```

The schema test rolls back all fixtures. Demo API tests create isolated workspaces and remove them on success. A failed test workspace expires normally. Ordinary cargo test does not contact Neon.

## Deployment direction

React static site and Rust web service on Render, PostgreSQL on Neon. Nothing has been deployed by this setup. Set HOST=0.0.0.0 on the Rust host, use its PORT, set FRONTEND_ORIGIN to the exact frontend origin, and store DATABASE_URL as a backend secret. Set VITE_API_BASE_URL to the Rust API origin before building the frontend. VITE_ values are public and must not contain secrets.

Apply migrations and seed explicitly before deploying the API. The API never migrates automatically. Public sessions are limited to thirty active workspaces; see docs/demo.md for expiry and cleanup behavior.

- docs/architecture.md: module boundaries and next milestones
- docs/database.md: schema, integrity constraints and workflow responsibilities
- docs/demo.md: dataset, session lifecycle, API routes and permission tests

- docs/workflow.md: document correction cycle, readiness rules, concurrency and API examples

- docs/analytics.md: follow-up actions, metric definitions, scope, drill-downs, CSV and tests
