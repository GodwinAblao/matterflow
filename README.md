# MatterFlow

Legal document readiness and operations analytics portfolio prototype.
Built with Rust, Axum, SQLx, PostgreSQL, React, TypeScript, and Vite.

Independent project inspired by publicly described legal-office workflows. It is not an official Salenga Law Firm system. Any future demo data will be fictional.

## Current scope

This initial setup provides the application structure, a Rust API with health/readiness endpoints, database connection configuration, and a React page that checks the live API. Matter workflows, authentication, document review, and analytics are not implemented yet.

## Requirements

- Node.js 24 LTS and npm
- Stable Rust with Cargo, rustfmt, and Clippy (install through https://rustup.rs)
- On Windows: Visual Studio C++ Build Tools and a Windows SDK
- PostgreSQL or a Neon connection string when database work begins

## Run locally

From the repository root:

~~~powershell
npm.cmd install
Copy-Item backend/.env.example backend/.env
npm.cmd run dev:api
~~~

Open a second terminal at the repository root:

~~~powershell
npm.cmd run dev
~~~

Open http://localhost:5173. Vite proxies /api requests to the Rust backend on port 3000. Using npm.cmd avoids PowerShell script execution-policy errors on Windows; npm works normally on other shells.

DATABASE_URL is optional for the initial framework. The API starts without it, but its readiness endpoint correctly returns 503 until a working database is configured. Backend configuration is read from backend/.env when launched from the repository root. Environment variables provided by the host take precedence.

## Structure

~~~text
matterflow/
  backend/
    src/
      app.rs             # Router, middleware, API fallback
      config.rs          # Environment configuration
      state.rs           # Shared database pool
      error.rs           # JSON API error responses
      db/                # PostgreSQL connection setup
      features/          # Feature modules; health is implemented
    migrations/          # Future versioned SQL migrations
    tests/               # HTTP integration checks
  frontend/
    src/
      components/        # Shared UI components
      features/          # Feature-specific UI; health is implemented
      lib/               # Shared HTTP client
  docs/                  # Architecture and implementation plan
~~~

## Verify

~~~powershell
npm.cmd run lint
npm.cmd run build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
~~~

## API

- GET /api/v1/health: process liveness; does not require PostgreSQL.
- GET /api/v1/ready: checks PostgreSQL connectivity; returns 503 when unconfigured or unavailable.

## Deployment direction

React static site and Rust web service on Render, PostgreSQL on Neon. No services have been deployed by this setup. Set HOST=0.0.0.0 on the Rust host, use the host-provided PORT, set FRONTEND_ORIGIN to the exact frontend origin, and set DATABASE_URL as a backend secret. Set VITE_API_BASE_URL to the backend origin before building the frontend. Never put database credentials in VITE_ variables, because those values are public.

See docs/architecture.md for module boundaries and next milestones.
