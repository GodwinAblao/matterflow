use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;

use crate::{error::ApiError, state::AppState};

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
}

pub async fn liveness() -> Json<HealthResponse> {
    response("ok")
}

pub async fn readiness(State(state): State<AppState>) -> Result<Json<HealthResponse>, ApiError> {
    let pool = state.pool.as_ref().ok_or(ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "database_not_configured",
        message: "The database has not been configured.",
        details: None,
    })?;
    sqlx::query("SELECT 1").execute(pool).await.map_err(|_| {
        tracing::warn!("Database readiness check failed");
        ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "database_unavailable",
            message: "The database is temporarily unavailable.",
            details: None,
        }
    })?;
    Ok(response("ready"))
}

fn response(status: &'static str) -> Json<HealthResponse> {
    Json(HealthResponse {
        status,
        service: "matterflow-api",
        version: env!("CARGO_PKG_VERSION"),
    })
}
