# PostgreSQL migrations

Run npm.cmd run db:migrate from the repository root after configuring backend/.env.
SQLx records applied versions and checksums in _sqlx_migrations. Applied migration files are immutable; add a later numbered migration for subsequent changes.

Migrations are embedded into the database command at build time. build.rs tracks this folder so new migrations trigger recompilation. API startup never applies migrations automatically.

The initial migration creates the matterflow schema and seven application tables. It does not add real or demonstration records. See docs/database.md for boundaries and verification.
