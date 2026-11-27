use axum::{
    Router,
    http::{HeaderValue, Method},
    routing::{get, post},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    error::ApiError,
    features::{auth::handler as auth, documents, health, matters},
    state::AppState,
};

pub fn create_router(
    state: AppState,
    frontend_origin: &str,
) -> Result<Router, axum::http::header::InvalidHeaderValue> {
    let origin: HeaderValue = frontend_origin.parse()?;
    let cors = CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ]);

    Ok(Router::new()
        .route("/api/v1/health", get(health::liveness))
        .route("/api/v1/ready", get(health::readiness))
        .route("/api/v1/demo/sessions", post(auth::create))
        .route(
            "/api/v1/demo/session",
            get(auth::current)
                .patch(auth::switch_role)
                .delete(auth::end),
        )
        .route("/api/v1/team", get(auth::team))
        .route("/api/v1/matters", get(matters::list))
        .route("/api/v1/matters/{id}", get(matters::detail))
        .route("/api/v1/matters/{id}/ready", post(matters::mark_ready))
        .route(
            "/api/v1/matters/{matter_id}/documents/{id}/receive",
            post(documents::receive),
        )
        .route(
            "/api/v1/matters/{matter_id}/documents/{id}/review",
            post(documents::review),
        )
        .fallback(not_found)
        .layer(axum::extract::DefaultBodyLimit::max(4096))
        .layer(cors)
        .layer(tower_http::set_header::SetResponseHeaderLayer::overriding(
            axum::http::header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state))
}

async fn not_found() -> ApiError {
    ApiError {
        status: axum::http::StatusCode::NOT_FOUND,
        code: "not_found",
        message: "The requested API endpoint does not exist.",
        details: None,
    }
}
