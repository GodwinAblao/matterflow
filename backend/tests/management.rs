mod support;
use axum::http::StatusCode;
use serde_json::{Value, json};
use support::Demo;
use uuid::Uuid;
fn draft() -> Value {
    json!({"creation_key":Uuid::from_u128(rand::random()),"title":"Business document reviewe","client_label":"Fictional CRUD Client","service_category":"business","documents":[{"title":"Synthetic requirement","is_required":true}]})
}
fn update(d: &Value) -> Value {
    json!({"expected_version":d["matter"]["version"],"title":"Business document review","client_label":"Corrected fictional client","service_category":"property","assigned_staff_id":d["matter"]["assigned_staff_id"]})
}
#[tokio::test]
#[ignore = "requires migrated and seeded development database"]
async fn edits_delete_visibility_and_reference_integrity() {
    let demo = Demo::new().await;
    let payload = draft();
    let (s, c) = demo.request("POST", "/matters", payload.clone()).await;
    assert_eq!(s, StatusCode::CREATED);
    let id = c["id"].as_str().unwrap();
    let path = format!("/matters/{id}");
    let initial = demo.detail(id).await;
    assert_eq!(initial["can_delete"], true);
    let edits = update(&initial);
    assert_eq!(
        demo.request("PATCH", &path, edits.clone()).await.0,
        StatusCode::OK
    );
    assert_eq!(
        demo.request("PATCH", &path, edits.clone()).await.0,
        StatusCode::CONFLICT
    );
    let changed = demo.detail(id).await;
    assert_eq!(changed["matter"]["title"], "Business document review");
    assert_eq!(changed["matter"]["reference"], c["reference"]);
    assert_eq!(changed["documents"], initial["documents"]);
    assert_eq!(changed["can_delete"], true);
    assert!(
        changed["activity"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["event_type"] == "matter.updated")
    );
    let mut invalid = update(&changed);
    invalid["title"] = json!("   ");
    assert_eq!(
        demo.request("PATCH", &path, invalid).await.0,
        StatusCode::BAD_REQUEST
    );
    let foreign = Demo::new().await;
    assert_eq!(
        foreign.request("PATCH", &path, update(&changed)).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        foreign
            .request(
                "DELETE",
                &path,
                json!({"expected_version":1,"reference":c["reference"]})
            )
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    foreign.finish().await;
    demo.role("reviewer").await;
    let options = demo.request("GET", "/matters/options", Value::Null).await.1;
    let other = options
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] != changed["matter"]["assigned_staff_id"])
        .unwrap();
    demo.role("staff").await;
    let mut reassigned = update(&changed);
    reassigned["assigned_staff_id"] = other["id"].clone();
    assert_eq!(
        demo.request("PATCH", &path, reassigned).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        demo.request(
            "DELETE",
            &path,
            json!({"expected_version":1,"reference":"MF-WRONG"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        demo.request(
            "DELETE",
            &path,
            json!({"expected_version":1,"reference":c["reference"]})
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        demo.request("GET", &path, Value::Null).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        demo.request("GET", "/matters", Value::Null)
            .await
            .1
            .as_array()
            .unwrap()
            .len(),
        6
    );
    let analytics = demo.request("GET", "/analytics", Value::Null).await.1;
    assert_eq!(analytics["metrics"]["visible_matters"], 6);
    assert!(
        !analytics["requirements"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["matter_id"] == c["id"])
    );
    assert_eq!(
        demo.request("POST", "/matters", payload).await.0,
        StatusCode::CONFLICT
    );
    let (s, next) = demo.request("POST", "/matters", draft()).await;
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(next["reference"], "MF-014");
    let audit:i64=sqlx::query_scalar("SELECT count(*) FROM matterflow.activity_events WHERE workspace_id=$2 AND matter_id=$1::uuid AND event_type='matter.deleted'").bind(id).bind(demo.workspace).fetch_one(&demo.pool).await.unwrap();
    assert_eq!(audit, 1);
    demo.finish().await;
}
#[tokio::test]
#[ignore = "requires migrated and seeded development database"]
async fn workflow_matters_require_reviewer_archiving_and_reassignment_moves_open_tasks() {
    let demo = Demo::new().await;
    let (_, c) = demo.request("POST", "/matters", draft()).await;
    let id = c["id"].as_str().unwrap();
    let path = format!("/matters/{id}");
    let (_, follow) = demo
        .request(
            "POST",
            &format!("{path}/follow-ups"),
            json!({"title":"Fictional open task","due_on":"2026-12-08"}),
        )
        .await;
    let d = demo.detail(id).await;
    assert_eq!(d["can_delete"], false);
    let confirm = json!({"expected_version":d["matter"]["version"],"reference":c["reference"]});
    assert_eq!(
        demo.request("DELETE", &path, confirm.clone()).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        demo.request("POST", &format!("{path}/archive"), confirm.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    demo.role("reviewer").await;
    let options = demo.request("GET", "/matters/options", Value::Null).await.1;
    let other = options
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] != d["matter"]["assigned_staff_id"])
        .unwrap();
    let mut changed = update(&d);
    changed["assigned_staff_id"] = other["id"].clone();
    assert_eq!(
        demo.request("PATCH", &path, changed).await.0,
        StatusCode::OK
    );
    let assigned: Uuid =
        sqlx::query_scalar("SELECT assigned_to FROM matterflow.follow_ups WHERE id=$1::uuid")
            .bind(follow["id"].as_str().unwrap())
            .fetch_one(&demo.pool)
            .await
            .unwrap();
    assert_eq!(assigned.to_string(), other["id"].as_str().unwrap());
    let d = demo.detail(id).await;
    let confirm = json!({"expected_version":d["matter"]["version"],"reference":c["reference"]});
    assert_eq!(
        demo.request("POST", &format!("{path}/archive"), confirm)
            .await
            .0,
        StatusCode::OK
    );
    let d = demo.detail(id).await;
    assert_eq!(d["matter"]["status"], "closed");
    assert_eq!(d["follow_ups"].as_array().unwrap().len(), 1);
    assert_eq!(
        demo.request("PATCH", &path, update(&d)).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        demo.request(
            "PATCH",
            &format!("{path}/follow-ups/{}", follow["id"].as_str().unwrap()),
            json!({"expected_version":1,"action":"complete","due_on":null})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let report = demo.request("GET", "/analytics", Value::Null).await.1;
    assert_eq!(report["metrics"]["visible_matters"], 13);
    assert_eq!(report["metrics"]["active_matters"], 10);
    demo.finish().await;
}
