use std::error::Error;

use matterflow_api::{app::create_router, config::Config, db, state::AppState};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    match dotenvy::from_filename("backend/.env") {
        Ok(_) => {}
        Err(dotenvy::Error::Io(ref error)) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::from_env()?;
    let pool = db::create_pool(config.database_url.as_deref())?;
    if pool.is_none() {
        tracing::warn!("DATABASE_URL is not set; readiness will report unavailable");
    }
    let app = create_router(AppState { pool }, &config.frontend_origin)?;
    let listener = tokio::net::TcpListener::bind((config.host.as_str(), config.port)).await?;
    tracing::info!(address = %listener.local_addr()?, "MatterFlow API listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(%error, "Could not listen for shutdown signal");
    }
}
