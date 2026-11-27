use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use matterflow_api::{app::create_router, state::AppState};
use tower::ServiceExt;

async fn request(path: &str) -> axum::response::Response {
    create_router(AppState { pool: None }, "http://localhost:5173")
        .expect("valid origin")
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn liveness_works_without_a_database() {
    let response = request("/api/v1/health").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("matterflow-api"));
}

#[tokio::test]
async fn readiness_is_unavailable_without_a_database() {
    let response = request("/api/v1/ready").await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    assert!(
        String::from_utf8(body.to_vec())
            .unwrap()
            .contains("database_not_configured")
    );
}

#[tokio::test]
async fn unknown_routes_return_json_errors() {
    let response = request("/api/v1/missing").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.headers()["content-type"], "application/json");
}
