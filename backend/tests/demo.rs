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

async fn database() -> PgPool {
    dotenvy::from_filename("backend/.env").ok();
    let url = std::env::var("DATABASE_URL").expect("configure the development database first");
    db::create_pool(Some(&url)).unwrap().unwrap()
}

async fn request(
    app: &Router,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
#[ignore = "requires migrated, seeded development database; run npm.cmd run test:demo"]
async fn demo_sessions_enforce_roles_expiry_and_workspace_isolation() {
    let pool = database().await;
    let app = create_router(
        AppState {
            pool: Some(pool.clone()),
        },
        "http://localhost:5173",
    )
    .unwrap();
    let (first, second) = tokio::join!(
        request(
            &app,
            "POST",
            "/api/v1/demo/sessions",
            None,
            json!({"role":"staff"})
        ),
        request(
            &app,
            "POST",
            "/api/v1/demo/sessions",
            None,
            json!({"role":"reviewer"})
        ),
    );
    assert_eq!(first.0, StatusCode::CREATED);
    assert_eq!(second.0, StatusCode::CREATED);
    let first_token = first.1["token"].as_str().unwrap();
    let second_token = second.1["token"].as_str().unwrap();
    let first_workspace: Uuid = first.1["session"]["workspace_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let second_workspace: Uuid = second.1["session"]["workspace_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert_ne!(first_workspace, second_workspace);
    let relative_dates: bool = sqlx::query_scalar("SELECT bool_and(due_on BETWEEN (now() AT TIME ZONE 'Asia/Manila')::date - 2 AND (now() AT TIME ZONE 'Asia/Manila')::date + 2) FROM matterflow.follow_ups WHERE workspace_id = $1")
        .bind(first_workspace).fetch_one(&pool).await.unwrap();
    assert!(relative_dates);
    let stored_hash: String = sqlx::query_scalar(
        "SELECT token_hash FROM matterflow.demo_sessions WHERE workspace_id = $1",
    )
    .bind(first_workspace)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stored_hash.len(), 64);
    assert!(stored_hash != first_token);
    assert_eq!(
        request(&app, "GET", "/api/v1/matters", None, Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(&app, "GET", "/api/v1/matters", Some("invalid"), Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(&app, "GET", "/api/v1/team", Some(first_token), Value::Null)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(&app, "GET", "/api/v1/team", Some(second_token), Value::Null)
            .await
            .0,
        StatusCode::OK
    );

    let staff = request(
        &app,
        "GET",
        "/api/v1/matters?role=reviewer",
        Some(first_token),
        Value::Null,
    )
    .await;
    assert_eq!(staff.0, StatusCode::OK);
    assert_eq!(staff.1.as_array().unwrap().len(), 6);
    let reviewer = request(
        &app,
        "GET",
        "/api/v1/matters",
        Some(second_token),
        Value::Null,
    )
    .await;
    assert_eq!(reviewer.1.as_array().unwrap().len(), 12);
    let foreign_id = reviewer.1[0]["id"].as_str().unwrap();
    assert_eq!(
        request(
            &app,
            "GET",
            &format!("/api/v1/matters/{foreign_id}"),
            Some(first_token),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let own_id = staff.1[0]["id"].as_str().unwrap();
    let detail = request(
        &app,
        "GET",
        &format!("/api/v1/matters/{own_id}"),
        Some(first_token),
        Value::Null,
    )
    .await;
    assert_eq!(detail.0, StatusCode::OK);
    assert_eq!(detail.1["documents"].as_array().unwrap().len(), 4);
    assert_eq!(detail.1["follow_ups"].as_array().unwrap().len(), 1);

    assert_eq!(
        request(
            &app,
            "PATCH",
            "/api/v1/demo/session",
            Some(first_token),
            json!({"role":"admin"})
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        request(
            &app,
            "PATCH",
            "/api/v1/demo/session",
            Some(first_token),
            json!({"role":"reviewer","workspace_id":second_workspace})
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let switched = request(
        &app,
        "PATCH",
        "/api/v1/demo/session",
        Some(first_token),
        json!({"role":"reviewer"}),
    )
    .await;
    assert_eq!(switched.0, StatusCode::OK);
    assert_eq!(
        switched.1["workspace_id"],
        first.1["session"]["workspace_id"]
    );
    assert_eq!(
        request(&app, "GET", "/api/v1/team", Some(first_token), Value::Null)
            .await
            .0,
        StatusCode::OK
    );
    let all_own = request(
        &app,
        "GET",
        "/api/v1/matters",
        Some(first_token),
        Value::Null,
    )
    .await;
    let staff_ids: Vec<&str> = staff
        .1
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap())
        .collect();
    let unassigned = all_own
        .1
        .as_array()
        .unwrap()
        .iter()
        .find(|row| !staff_ids.contains(&row["id"].as_str().unwrap()))
        .unwrap()["id"]
        .as_str()
        .unwrap();
    assert_eq!(
        request(
            &app,
            "PATCH",
            "/api/v1/demo/session",
            Some(first_token),
            json!({"role":"staff"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(
            &app,
            "GET",
            &format!("/api/v1/matters/{unassigned}"),
            Some(first_token),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );

    sqlx::query("UPDATE matterflow.demo_sessions SET created_at = now() - interval '2 hours', expires_at = now() - interval '1 second' WHERE workspace_id = $1")
        .bind(first_workspace).execute(&pool).await.unwrap();
    assert_eq!(
        request(
            &app,
            "GET",
            "/api/v1/demo/session",
            Some(first_token),
            Value::Null
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &app,
            "DELETE",
            "/api/v1/demo/session",
            Some(second_token),
            Value::Null
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            &app,
            "GET",
            "/api/v1/matters",
            Some(second_token),
            Value::Null
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    sqlx::query("DELETE FROM matterflow.workspaces WHERE id = $1 AND NOT is_template")
        .bind(first_workspace)
        .execute(&pool)
        .await
        .unwrap();
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM matterflow.workspaces WHERE id = ANY($1)")
            .bind(vec![first_workspace, second_workspace])
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(remaining, 0);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires migrated development database; run npm.cmd run test:demo"]
async fn fictional_seed_is_repeatable_and_ready_matters_have_accepted_requirements() {
    let pool = database().await;
    for _ in 0..2 {
        sqlx::raw_sql(include_str!("../seeds/demo.sql"))
            .execute(&pool)
            .await
            .unwrap();
    }
    let (users, matters, documents): (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM matterflow.app_users WHERE workspace_id=w.id),
                (SELECT count(*) FROM matterflow.matters WHERE workspace_id=w.id),
                (SELECT count(*) FROM matterflow.document_requirements WHERE workspace_id=w.id)
         FROM matterflow.workspaces w WHERE template_key='portfolio-v1'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((users, matters, documents), (3, 12, 48));
    let invalid_ready: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM matterflow.document_requirements d JOIN matterflow.matters m ON m.id=d.matter_id
         JOIN matterflow.workspaces w ON w.id=m.workspace_id WHERE w.template_key='portfolio-v1'
         AND m.status IN ('ready','closed') AND d.is_required AND d.status <> 'accepted'",
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(invalid_ready, 0);
    pool.close().await;
}
