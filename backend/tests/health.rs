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

#[tokio::test]
async fn browser_preflight_allows_demo_requests_from_the_configured_frontend() {
    let app = create_router(AppState { pool: None }, "http://localhost:5173").unwrap();
    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/v1/demo/session")
                .header("origin", "http://localhost:5173")
                .header("access-control-request-method", "PATCH")
                .header(
                    "access-control-request-headers",
                    "content-type,authorization",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["access-control-allow-origin"],
        "http://localhost:5173"
    );
    assert!(
        response.headers()["access-control-allow-methods"]
            .to_str()
            .unwrap()
            .contains("PATCH")
    );
}
