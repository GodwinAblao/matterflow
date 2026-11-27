# Free Render deployment

## Prepared configuration

render.yaml defines a free Rust web service in Singapore and a free React static site. PostgreSQL stays on the existing Neon project; this Blueprint does not create a Render database, disk, worker, or paid instance. Auto-deploys are off so future pushes do not publish changes unexpectedly.

The expected URLs are:

- Frontend: https://godwinablao-matterflow.onrender.com
- Rust API: https://godwinablao-matterflow-api.onrender.com

These are configuration defaults, not proof that a deployment exists. Check the actual URLs Render assigns. If it adds a suffix, update the backend FRONTEND_ORIGIN and frontend VITE_API_BASE_URL to the actual URLs before testing. Do not add a trailing slash or /api/v1 to either variable.

## Before creating services

1. Run npm.cmd run test:ui, npm.cmd run lint, npm.cmd run build, and npm.cmd run format:check locally.
2. Verify the intended Neon database with npm.cmd run db:check. Apply migrations and seed with npm.cmd run db:migrate and npm.cmd run db:seed. The existing development database has already been migrated, but a different database requires these explicit commands.
3. Commit and push the code, lockfiles, and render.yaml to GitHub. Never commit backend/.env. The hosting provider builds from the pushed commit, not your local working tree.

## Blueprint setup

1. Open https://dashboard.render.com/ and choose New > Blueprint.
2. Select GodwinAblao/matterflow, branch main, and the root render.yaml.
3. Review the resource list. The API must show the Free instance plan; the frontend must be a Static Site. No Render database is needed.
4. Enter DATABASE_URL when Render prompts. Copy its value directly from backend/.env, preserving the Neon SSL options. This secret belongs only on the Rust backend. Do not place it in frontend settings, screenshots, Git, or chat.
5. Apply the Blueprint and wait for both initial builds. Verify actual public URLs against the two configured URL variables. The Rust build is compiled on Render's Linux environment; a successful Windows build alone does not verify that hosted build.

The checked-in configuration uses Render's native Rust runtime. Build command: cargo build --locked --release -p matterflow-api --bin matterflow-api. Start command: ./target/release/matterflow-api. The host supplies PORT; HOST is 0.0.0.0. The API health check is /api/v1/health.

The frontend build runs npm ci --include=dev && npm run build from the repository root and publishes frontend/dist. A public API URL is compiled into the frontend by Vite. Changing VITE_API_BASE_URL requires a fresh frontend build.

## Manual setup alternative

If creating services individually, use exactly the same names, runtime, build/start commands, and variables as render.yaml. Leave Root Directory empty for both services because Cargo and npm workspaces are configured at the repository root. Choose Rust and the Free plan for the API, and Static Site for the frontend.

FRONTEND_ORIGIN is the exact frontend origin used by CORS. A mismatch causes browser requests to fail even if the API health endpoint works. Preserve the HTTPS scheme. Verify /api/v1/ready separately; it checks Neon connectivity, whereas /api/v1/health only checks that the API process responds.

## Free hosting behavior

Render's free API sleeps after fifteen minutes without inbound traffic and can take about a minute to wake. The static page can still load during that time. The entry page shows a service-starting message and offers retry; application requests allow up to seventy-five seconds before timing out. Free services are subject to provider usage limits and may restart.

Free Render web services have no persistent local disk, shell, or pre-deploy command. Run database migrations explicitly from your local machine against the intended Neon database before deploying a schema change. The API never applies migrations automatically. All application state is in PostgreSQL, so API restarts do not erase active demo workspaces.

## Verify the public application

- Open the public frontend from a fresh tab and wait for the service-ready message.
- Start as Staff, select MF-001, record a receipt, switch to Reviewer, and confirm that the same matter remains selected.
- Select a reviewer-only matter, switch to Staff, and verify its details disappear.
- Create a fictional matter and checklist, complete a correction cycle, and confirm readiness.
- Complete an overdue follow-up and refresh Analytics to verify its overdue count changes.
- Test category filters, chart drill-downs, CSV export, project overview, mobile layout, keyboard navigation, refresh/session restoration, and End demo.
- Test after idle sleep as well as while the backend is warm. Record the final public frontend URL for the application email.

Current local checks do not replace this hosted browser verification. A site is ready to submit only after these public-link checks pass.

## Reference documentation

- https://render.com/docs/blueprint-spec
- https://render.com/docs/free
- https://render.com/docs/deploys

## DATABASE_URL startup diagnostics

If a previous deploy exited with Configuration(RelativeUrlWithoutBase), check that the latest backend commit is deployed. Database configuration now trims outer whitespace, tolerates a complete DATABASE_URL= assignment and one pair of matching outer quotes, and validates the value with SQLx before building the pool. The raw PostgreSQL URL is still the recommended Render value.

A malformed value now produces an explicit Invalid DATABASE_URL startup message without echoing the value or parser error. A psql command, placeholder, variable reference, wrong scheme, or multiline value is rejected. A URL-format error occurs before port binding, so the accompanying no-open-ports warning is a consequence rather than the underlying issue.

After committing and pushing a backend fix, use Manual Deploy > Deploy latest commit on the API service. Auto-deploys are disabled in this Blueprint. Confirm the deploy's commit matches the new Git commit, then check /api/v1/health and /api/v1/ready.
