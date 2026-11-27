use axum::{
    Router,
    http::{HeaderValue, Method},
    routing::get,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{error::ApiError, features::health, state::AppState};

pub fn create_router(
    state: AppState,
    frontend_origin: &str,
) -> Result<Router, axum::http::header::InvalidHeaderValue> {
    let origin: HeaderValue = frontend_origin.parse()?;
    let cors = CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([Method::GET])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    Ok(Router::new()
        .route("/api/v1/health", get(health::liveness))
        .route("/api/v1/ready", get(health::readiness))
        .fallback(not_found)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state))
}

async fn not_found() -> ApiError {
    ApiError {
        status: axum::http::StatusCode::NOT_FOUND,
        code: "not_found",
        message: "The requested API endpoint does not exist.",
    }
}
