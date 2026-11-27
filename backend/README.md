# MatterFlow API

Rust + Axum with Tokio, Serde, SQLx, PostgreSQL, and tower-http middleware.

Run from the repository root:

~~~powershell
Copy-Item backend/.env.example backend/.env
cargo run -p matterflow-api
~~~

Default address: http://127.0.0.1:3000. A database is optional for initial startup. GET /api/v1/health reports process liveness; GET /api/v1/ready returns 503 until DATABASE_URL points to a reachable PostgreSQL database.

Windows requires the Visual Studio C++ Build Tools, including the MSVC x64/x86 compiler tools and a Windows SDK. If Cargo reports that link.exe is missing, install the Desktop development with C++ workload using Visual Studio Installer and reopen the terminal.

Feature modules live in src/features. See ../docs/architecture.md for planned handler, service, repository, and model boundaries.
