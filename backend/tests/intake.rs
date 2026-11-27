mod support;
use axum::http::StatusCode;
use serde_json::{Value, json};
use support::Demo;
use uuid::Uuid;
fn draft(key: u128) -> Value {
    json!({"creation_key":Uuid::from_u128(key),"title":"Fictional intake matter","client_label":"Demo Intake Client","service_category":"property","documents":[{"title":"Identity copy","is_required":true},{"title":"Additional notes","is_required":false}]})
}

#[tokio::test]
#[ignore = "requires migrated seeded development database; run npm.cmd run test:intake"]
async fn new_matter_checklist_and_review_form_a_complete_workflow() {
    let demo = Demo::new().await;
    let body = draft(1);
    let (status, created) = demo.request("POST", "/matters", body.clone()).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["reference"], "MF-013");
    let id = created["id"].as_str().unwrap();
    let replay = demo.request("POST", "/matters", body.clone()).await;
    assert_eq!(replay.0, StatusCode::OK);
    assert_eq!(replay.1["id"], created["id"]);
    assert_eq!(replay.1["replayed"], true);
    let mut changed = body.clone();
    changed["title"] = json!("Different payload");
    assert_eq!(
        demo.request("POST", "/matters", changed).await.0,
        StatusCode::CONFLICT
    );
    let stored_workspace: Uuid =
        sqlx::query_scalar("SELECT workspace_id FROM matterflow.matters WHERE id=$1::uuid")
            .bind(id)
            .fetch_one(&demo.pool)
            .await
            .unwrap();
    assert_eq!(stored_workspace, demo.workspace);
    let initial = demo.detail(id).await;
    assert_eq!(initial["documents"].as_array().unwrap().len(), 2);
    assert_eq!(initial["activity"].as_array().unwrap().len(), 1);
    assert_eq!(
        demo.request("GET", "/analytics", Value::Null).await.1["metrics"]["visible_matters"],
        7
    );
    let added = demo
        .request(
            "POST",
            &format!("/matters/{id}/requirements"),
            json!({"expected_version":0,"title":" Supporting   copy ","is_required":true}),
        )
        .await;
    assert_eq!(added.0, StatusCode::CREATED);
    assert_eq!(
        demo.request(
            "POST",
            &format!("/matters/{id}/requirements"),
            json!({"expected_version":0,"title":"Stale change","is_required":true})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        demo.request(
            "POST",
            &format!("/matters/{id}/requirements"),
            json!({"expected_version":1,"title":"IDENTITY COPY","is_required":true})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let detail = demo.detail(id).await;
    let required = detail["documents"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["is_required"] == true)
        .map(|row| row["id"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(required.len(), 2);
    for document in &required {
        assert_eq!(
            demo.request(
                "POST",
                &format!("/matters/{id}/documents/{document}/receive"),
                json!({"expected_version":0})
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    demo.role("reviewer").await;
    for document in &required {
        assert_eq!(
            demo.request(
                "POST",
                &format!("/matters/{id}/documents/{document}/review"),
                json!({"expected_version":1,"outcome":"accepted","note":null})
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    let complete = demo.detail(id).await;
    assert_eq!(complete["readiness"]["documents_complete"], true);
    assert_eq!(
        demo.request(
            "POST",
            &format!("/matters/{id}/ready"),
            json!({"expected_version":complete["matter"]["version"]})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(demo.request("POST",&format!("/matters/{id}/requirements"),json!({"expected_version":complete["matter"]["version"],"title":"Too late","is_required":true})).await.0,StatusCode::CONFLICT);
    let foreign = Demo::new().await;
    assert_eq!(
        foreign
            .request(
                "POST",
                &format!("/matters/{id}/requirements"),
                json!({"expected_version":0,"title":"Foreign edit","is_required":true})
            )
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    foreign.finish().await;
    demo.finish().await;
}

#[tokio::test]
#[ignore = "requires migrated seeded development database; run npm.cmd run test:intake"]
async fn creation_validation_assignment_and_concurrent_retries_are_safe() {
    let demo = Demo::new().await;
    let mut invalid = draft(1);
    invalid["documents"] = json!([]);
    assert_eq!(
        demo.request("POST", "/matters", invalid).await.0,
        StatusCode::BAD_REQUEST
    );
    let mut invalid = draft(1);
    invalid["documents"][0]["is_required"] = json!(false);
    assert_eq!(
        demo.request("POST", "/matters", invalid).await.0,
        StatusCode::BAD_REQUEST
    );
    let mut invalid = draft(1);
    invalid["documents"][1]["title"] = json!("  IDENTITY   copy  ");
    assert_eq!(
        demo.request("POST", "/matters", invalid).await.0,
        StatusCode::BAD_REQUEST
    );
    let options = demo.request("GET", "/matters/options", Value::Null).await;
    assert_eq!(options.1.as_array().unwrap().len(), 1);
    let foreign = Demo::new().await;
    let (_, foreign_options) = foreign
        .request("GET", "/matters/options", Value::Null)
        .await;
    let mut invalid = draft(1);
    invalid["assigned_staff_id"] = foreign_options[0]["id"].clone();
    assert_eq!(
        demo.request("POST", "/matters", invalid.clone()).await.0,
        StatusCode::FORBIDDEN
    );
    demo.role("reviewer").await;
    assert_eq!(
        demo.request("POST", "/matters", invalid).await.0,
        StatusCode::BAD_REQUEST
    );
    demo.role("staff").await;
    let (a, b) = tokio::join!(
        demo.request("POST", "/matters", draft(2)),
        demo.request("POST", "/matters", draft(2))
    );
    assert!(
        (a.0 == StatusCode::CREATED && b.0 == StatusCode::OK)
            || (b.0 == StatusCode::CREATED && a.0 == StatusCode::OK)
    );
    assert_eq!(a.1["id"], b.1["id"]);
    let (a, b) = tokio::join!(
        demo.request("POST", "/matters", draft(3)),
        demo.request("POST", "/matters", draft(4))
    );
    assert_eq!(a.0, StatusCode::CREATED);
    assert_eq!(b.0, StatusCode::CREATED);
    assert_ne!(a.1["reference"], b.1["reference"]);
    assert_eq!(
        demo.request("GET", "/matters", Value::Null)
            .await
            .1
            .as_array()
            .unwrap()
            .len(),
        9
    );
    foreign.finish().await;
    demo.finish().await;
}

#[tokio::test]
#[ignore = "requires migrated seeded development database; run npm.cmd run test:intake"]
async fn adding_a_requirement_cannot_race_past_readiness_confirmation() {
    let demo = Demo::new().await;
    let (_, created) = demo.request("POST", "/matters", draft(1)).await;
    let id = created["id"].as_str().unwrap();
    let detail = demo.detail(id).await;
    let doc = detail["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["is_required"] == true)
        .unwrap()["id"]
        .as_str()
        .unwrap();
    assert_eq!(
        demo.request(
            "POST",
            &format!("/matters/{id}/documents/{doc}/receive"),
            json!({"expected_version":0})
        )
        .await
        .0,
        StatusCode::OK
    );
    demo.role("reviewer").await;
    assert_eq!(
        demo.request(
            "POST",
            &format!("/matters/{id}/documents/{doc}/review"),
            json!({"expected_version":1,"outcome":"accepted","note":null})
        )
        .await
        .0,
        StatusCode::OK
    );
    let add_path = format!("/matters/{id}/requirements");
    let ready_path = format!("/matters/{id}/ready");
    let (add, ready) = tokio::join!(
        demo.request(
            "POST",
            &add_path,
            json!({"expected_version":2,"title":"New mandatory requirement","is_required":true})
        ),
        demo.request("POST", &ready_path, json!({"expected_version":2}))
    );
    assert!(
        (add.0 == StatusCode::CREATED && ready.0 == StatusCode::CONFLICT)
            || (add.0 == StatusCode::CONFLICT && ready.0 == StatusCode::OK)
    );
    let final_state = demo.detail(id).await;
    assert!(
        final_state["matter"]["status"] != "ready"
            || final_state["readiness"]["documents_complete"] == true
    );
    demo.finish().await;
}
