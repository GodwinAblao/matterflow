use std::process::ExitCode;

use matterflow_api::{config::Config, db};

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), &'static str> {
    let command = std::env::args().nth(1).unwrap_or_default();
    if !matches!(command.as_str(), "migrate" | "check" | "seed" | "cleanup") {
        return Err(
            "Usage: cargo run -p matterflow-api --bin matterflow-db -- <migrate|check|seed|cleanup>",
        );
    }
    match dotenvy::from_filename("backend/.env") {
        Ok(_) => {}
        Err(dotenvy::Error::Io(ref error)) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("Could not load backend/.env. Check its format and permissions."),
    }
    let config = Config::from_env().map_err(|_| "Invalid backend configuration.")?;
    let pool = db::create_pool(config.database_url.as_deref())
        .map_err(|_| "Invalid DATABASE_URL. Credentials have not been displayed.")?
        .ok_or("Set DATABASE_URL in backend/.env first.")?;
    sqlx::query("SELECT 1").execute(&pool).await.map_err(
        |_| "Database connection failed. Check the connection string, SSL settings, and network.",
    )?;
    println!("PostgreSQL connection verified.");

    if command == "migrate" {
        db::MIGRATOR.run(&pool).await
            .map_err(|_| "Migration failed. Check database permissions, schema conflicts, and migration checksums.")?;
        println!("All embedded migrations applied successfully.");
    }
    if command == "seed" {
        sqlx::raw_sql(include_str!("../../seeds/demo.sql"))
            .execute(&pool)
            .await
            .map_err(|error| {
                if let Some(database) = error.as_database_error() {
                    eprintln!(
                        "Seed SQL error {:?}; constraint {:?}",
                        database.code(),
                        database.constraint()
                    );
                }
                "Could not seed the fictional template. Apply migrations first."
            })?;
        println!("Fictional template is ready (existing templates are preserved).");
    }
    if command == "cleanup" {
        let result = sqlx::query(
            "DELETE FROM matterflow.workspaces WHERE NOT is_template AND expires_at <= now()",
        )
        .execute(&pool)
        .await
        .map_err(|_| "Could not remove expired demo workspaces.")?;
        println!(
            "Expired demo workspaces removed: {}",
            result.rows_affected()
        );
    }
    let table_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.tables WHERE table_schema = 'matterflow' AND table_type = 'BASE TABLE'",
    ).fetch_one(&pool).await.map_err(|_| "Could not inspect the MatterFlow schema.")?;
    println!("MatterFlow tables: {table_count}");
    pool.close().await;
    Ok(())
}
