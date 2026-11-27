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

struct Demo {
    app: Router,
    pool: PgPool,
    token: String,
    workspace: Uuid,
}
impl Demo {
    async fn new() -> Self {
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
    async fn request(&self, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
        call(&self.app, Some(&self.token), method, path, body).await
    }
    async fn role(&self, role: &str) {
        assert_eq!(
            self.request("PATCH", "/demo/session", json!({"role":role}))
                .await
                .0,
            StatusCode::OK
        );
    }
    async fn first(&self) -> String {
        let (_, rows) = self.request("GET", "/matters", Value::Null).await;
        rows[0]["id"].as_str().unwrap().into()
    }
    async fn detail(&self, id: &str) -> Value {
        let (status, body) = self
            .request("GET", &format!("/matters/{id}"), Value::Null)
            .await;
        assert_eq!(status, StatusCode::OK);
        body
    }
    async fn receive(&self, matter: &str, id: &str, version: i64) -> (StatusCode, Value) {
        self.request(
            "POST",
            &format!("/matters/{matter}/documents/{id}/receive"),
            json!({"expected_version":version}),
        )
        .await
    }
    async fn review(
        &self,
        matter: &str,
        id: &str,
        version: i64,
        outcome: &str,
        note: Option<&str>,
    ) -> (StatusCode, Value) {
        self.request(
            "POST",
            &format!("/matters/{matter}/documents/{id}/review"),
            json!({"expected_version":version,"outcome":outcome,"note":note}),
        )
        .await
    }
    async fn ready(&self, matter: &str, version: i64) -> (StatusCode, Value) {
        self.request(
            "POST",
            &format!("/matters/{matter}/ready"),
            json!({"expected_version":version}),
        )
        .await
    }
    async fn finish(self) {
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
fn required_ids(detail: &Value) -> Vec<String> {
    detail["documents"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["is_required"] == true)
        .map(|row| row["id"].as_str().unwrap().into())
        .collect()
}
fn document<'a>(detail: &'a Value, id: &str) -> &'a Value {
    detail["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == id)
        .unwrap()
}

#[tokio::test]
#[ignore = "requires migrated and seeded development database; run npm.cmd run test:workflow"]
async fn correction_cycle_enforces_permissions_blockers_versions_and_atomic_history() {
    let demo = Demo::new().await;
    let matter = demo.first().await;
    let initial = demo.detail(&matter).await;
    let docs = required_ids(&initial);
    assert_eq!(docs.len(), 3);
    assert_eq!(demo.ready(&matter, 0).await.0, StatusCode::FORBIDDEN);
    assert_eq!(
        demo.review(&matter, &docs[0], 0, "accepted", None).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(demo.receive(&matter, &docs[0], 0).await.0, StatusCode::OK);
    assert_eq!(
        demo.receive(&matter, &docs[0], 0).await.0,
        StatusCode::CONFLICT
    );
    demo.role("reviewer").await;
    assert_eq!(
        demo.receive(&matter, &docs[1], 0).await.0,
        StatusCode::FORBIDDEN
    );
    let blocked = demo.ready(&matter, 1).await;
    assert_eq!(blocked.0, StatusCode::CONFLICT);
    assert_eq!(blocked.1["error"]["code"], "readiness_blocked");
    assert_eq!(
        blocked.1["error"]["details"]["blockers"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        demo.review(&matter, &docs[1], 0, "accepted", None).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        demo.review(&matter, &docs[0], 1, "needs_correction", Some("  "))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        demo.review(
            &matter,
            &docs[0],
            1,
            "needs_correction",
            Some("Please provide a readable copy.")
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        demo.review(&matter, &docs[0], 1, "accepted", None).await.0,
        StatusCode::CONFLICT
    );
    demo.role("staff").await;
    assert_eq!(demo.receive(&matter, &docs[0], 2).await.0, StatusCode::OK);
    let resubmitted = demo.detail(&matter).await;
    assert!(document(&resubmitted, &docs[0])["correction_note"].is_null());
    let (a, b) = tokio::join!(
        demo.receive(&matter, &docs[1], 0),
        demo.receive(&matter, &docs[2], 0)
    );
    assert_eq!(a.0, StatusCode::OK);
    assert_eq!(b.0, StatusCode::OK);
    demo.role("reviewer").await;
    for id in &docs {
        let state = demo.detail(&matter).await;
        assert_eq!(
            demo.review(
                &matter,
                id,
                document(&state, id)["version"].as_i64().unwrap(),
                "accepted",
                None
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    let complete = demo.detail(&matter).await;
    assert_eq!(complete["readiness"]["documents_complete"], true);
    // Optional documents remain missing and do not block readiness.
    assert!(
        complete["documents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["is_required"] == false && row["status"] == "missing")
    );
    assert_eq!(demo.ready(&matter, 0).await.0, StatusCode::CONFLICT);
    let version = complete["matter"]["version"].as_i64().unwrap();
    assert_eq!(demo.ready(&matter, version).await.0, StatusCode::OK);
    assert_eq!(demo.ready(&matter, version).await.0, StatusCode::OK);
    let ready = demo.detail(&matter).await;
    assert_eq!(ready["matter"]["status"], "ready");
    assert_eq!(ready["activity"].as_array().unwrap().len(), 10);
    assert_eq!(
        ready["activity"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["event_type"] == "matter.ready")
            .count(),
        1
    );
    assert!(
        ready["activity"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["event_type"] == "document.correction_requested"
                && row["details"]["note"] == "Please provide a readable copy.")
    );
    for row in ready["documents"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["is_required"] == true)
    {
        assert!(row["reviewed_at"].as_str().unwrap() >= row["received_at"].as_str().unwrap());
    }
    demo.role("staff").await;
    let optional = ready["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["is_required"] == false)
        .unwrap()["id"]
        .as_str()
        .unwrap();
    assert_eq!(
        demo.receive(&matter, optional, 0).await.0,
        StatusCode::CONFLICT
    );
    let foreign = Demo::new().await;
    let foreign_matter = foreign.first().await;
    assert_eq!(
        demo.receive(&foreign_matter, &docs[0], 0).await.0,
        StatusCode::NOT_FOUND
    );
    foreign.finish().await;
    demo.finish().await;
}

#[tokio::test]
#[ignore = "requires migrated and seeded development database; run npm.cmd run test:workflow"]
async fn concurrent_changes_cannot_duplicate_receipt_or_bypass_readiness() {
    let demo = Demo::new().await;
    let matter = demo.first().await;
    let initial = demo.detail(&matter).await;
    let docs = required_ids(&initial);
    let (a, b) = tokio::join!(
        demo.receive(&matter, &docs[0], 0),
        demo.receive(&matter, &docs[0], 0)
    );
    assert!(
        (a.0 == StatusCode::OK && b.0 == StatusCode::CONFLICT)
            || (b.0 == StatusCode::OK && a.0 == StatusCode::CONFLICT)
    );
    let received = demo.detail(&matter).await;
    assert_eq!(
        received["activity"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["event_type"] == "document.received")
            .count(),
        1
    );
    for id in &docs[1..] {
        assert_eq!(demo.receive(&matter, id, 0).await.0, StatusCode::OK);
    }
    demo.role("reviewer").await;
    for id in &docs[..2] {
        assert_eq!(
            demo.review(&matter, id, 1, "accepted", None).await.0,
            StatusCode::OK
        );
    }
    let pending = demo.detail(&matter).await;
    let version = pending["matter"]["version"].as_i64().unwrap();
    let (review, ready) = tokio::join!(
        demo.review(&matter, &docs[2], 1, "accepted", None),
        demo.ready(&matter, version)
    );
    assert_eq!(review.0, StatusCode::OK);
    assert_eq!(ready.0, StatusCode::CONFLICT);
    let complete = demo.detail(&matter).await;
    assert_eq!(complete["matter"]["status"], "under_review");
    assert_eq!(complete["readiness"]["documents_complete"], true);
    // An empty required checklist is never treated as complete.
    let (_, matters) = demo.request("GET", "/matters", Value::Null).await;
    let other = matters
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] != matter && row["status"] != "ready" && row["status"] != "closed")
        .unwrap()["id"]
        .as_str()
        .unwrap();
    sqlx::query("UPDATE matterflow.document_requirements SET is_required=FALSE WHERE workspace_id=$1 AND matter_id=$2::uuid").bind(demo.workspace).bind(other).execute(&demo.pool).await.unwrap();
    let empty = demo.detail(other).await;
    assert_eq!(empty["readiness"]["documents_complete"], false);
    let result = demo
        .ready(other, empty["matter"]["version"].as_i64().unwrap())
        .await;
    assert_eq!(result.0, StatusCode::CONFLICT);
    assert_eq!(
        result.1["error"]["details"]["blockers"][0]["status"],
        "empty"
    );
    demo.finish().await;
}
