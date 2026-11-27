use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use matterflow_api::{app::create_router, db, state::AppState};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

pub struct Demo {
    pub app: Router,
    pub pool: PgPool,
    pub token: String,
    pub workspace: Uuid,
}
impl Demo {
    pub async fn new() -> Self {
        dotenvy::from_filename("backend/.env").ok();
        let url = std::env::var("DATABASE_URL").expect("configure the development database");
        let pool = db::create_pool(Some(&url)).unwrap().unwrap();
        let app = create_router(
            AppState {
                pool: Some(pool.clone()),
            },
            "http://localhost:5173",
        )
        .unwrap();
        let (status, body) = call(
            &app,
            None,
            "POST",
            "/demo/sessions",
            json!({"role":"staff"}),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        Self {
            app,
            pool,
            token: body["token"].as_str().unwrap().into(),
            workspace: body["session"]["workspace_id"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap(),
        }
    }
    pub async fn request(&self, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
        call(&self.app, Some(&self.token), method, path, body).await
    }
    pub async fn role(&self, role: &str) {
        assert_eq!(
            self.request("PATCH", "/demo/session", json!({"role":role}))
                .await
                .0,
            StatusCode::OK
        );
    }

    pub async fn detail(&self, id: &str) -> Value {
        let (status, body) = self
            .request("GET", &format!("/matters/{id}"), Value::Null)
            .await;
        assert_eq!(status, StatusCode::OK);
        body
    }

    pub async fn finish(self) {
        assert_eq!(
            self.request("DELETE", "/demo/session", Value::Null).await.0,
            StatusCode::NO_CONTENT
        );
        self.pool.close().await;
    }
}
async fn call(
    app: &Router,
    token: Option<&str>,
    method: &str,
    path: &str,
    body: Value,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri(format!("/api/v1{path}"))
        .header("content-type", "application/json");
    if let Some(token) = token {
        req = req.header("authorization", format!("Bearer {token}"));
    }
    let response = app
        .clone()
        .oneshot(req.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 1_000_000).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}
