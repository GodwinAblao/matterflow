mod support;
use axum::http::StatusCode;
use serde_json::{Value, json};
use support::Demo;

#[tokio::test]
#[ignore = "requires migrated and seeded development database; run npm.cmd run test:analytics"]
async fn report_calculations_and_follow_up_actions_use_scoped_records_and_manila_dates() {
    let demo = Demo::new().await;
    let (_, rows) = demo.request("GET", "/matters", Value::Null).await;
    let first = rows[0]["id"].as_str().unwrap();
    let second = rows[1]["id"].as_str().unwrap();
    // A controlled two-matter cohort: one unfinished, one ready after three days.
    sqlx::query("UPDATE matterflow.matters SET service_category='criminal' WHERE workspace_id=$1 AND id IN($2::uuid,$3::uuid)").bind(demo.workspace).bind(first).bind(second).execute(&demo.pool).await.unwrap();
    sqlx::query("UPDATE matterflow.document_requirements SET status='accepted',received_at=requested_at+interval '1 hour',reviewed_at=requested_at+interval '2 hours',reviewed_by=(SELECT id FROM matterflow.app_users WHERE workspace_id=$1 AND role='reviewer'),correction_note=NULL WHERE workspace_id=$1 AND matter_id=$2::uuid AND is_required").bind(demo.workspace).bind(second).execute(&demo.pool).await.unwrap();
    sqlx::query("UPDATE matterflow.matters SET status='ready',first_ready_at=created_at+interval '3 days' WHERE workspace_id=$1 AND id=$2::uuid").bind(demo.workspace).bind(second).execute(&demo.pool).await.unwrap();
    sqlx::query("UPDATE matterflow.follow_ups SET due_on=(now() AT TIME ZONE 'Asia/Manila')::date-1 WHERE workspace_id=$1 AND matter_id=$2::uuid").bind(demo.workspace).bind(first).execute(&demo.pool).await.unwrap();
    sqlx::query("UPDATE matterflow.follow_ups SET completed_at=clock_timestamp() WHERE workspace_id=$1 AND matter_id=$2::uuid").bind(demo.workspace).bind(second).execute(&demo.pool).await.unwrap();
    let report = demo
        .request("GET", "/analytics?category=criminal", Value::Null)
        .await;
    assert_eq!(report.0, StatusCode::OK);
    let metrics = &report.1["metrics"];
    assert_eq!(metrics["visible_matters"], 2);
    assert_eq!(metrics["required_documents"], 6);
    assert_eq!(metrics["accepted_documents"], 3);
    assert_eq!(metrics["completeness_percent"], 50.0);
    assert_eq!(metrics["outstanding_requirements"], 3);
    assert_eq!(metrics["median_days_to_readiness"], 3.0);
    assert_eq!(metrics["readiness_cohort"], 1);
    assert_eq!(metrics["overdue_follow_ups"], 1);
    let today = report.1["local_date"].as_str().unwrap();
    let created = demo
        .request(
            "POST",
            &format!("/matters/{first}/follow-ups"),
            json!({"title":"Today boundary fixture","due_on":today}),
        )
        .await;
    assert_eq!(created.0, StatusCode::OK);
    let task = created.1["id"].as_str().unwrap();
    let after = demo
        .request("GET", "/analytics?category=criminal", Value::Null)
        .await;
    assert_eq!(after.1["metrics"]["overdue_follow_ups"], 1); // Due today is not overdue.
    let old_task = report.1["follow_ups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["overdue"] == true)
        .unwrap()["id"]
        .as_str()
        .unwrap();
    let path = format!("/matters/{first}/follow-ups/{old_task}");
    let complete = demo
        .request(
            "PATCH",
            &path,
            json!({"expected_version":0,"action":"complete","due_on":null}),
        )
        .await;
    assert_eq!(complete.0, StatusCode::OK);
    assert_eq!(
        demo.request(
            "PATCH",
            &path,
            json!({"expected_version":1,"action":"reschedule","due_on":today})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        demo.request(
            "PATCH",
            &path,
            json!({"expected_version":1,"action":"reschedule","due_on":null})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        demo.request(
            "POST",
            &format!("/matters/{first}/follow-ups"),
            json!({"title":"  ","due_on":today})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );

    assert_eq!(
        demo.request(
            "PATCH",
            &path,
            json!({"expected_version":0,"action":"complete","due_on":null})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        demo.request("GET", "/analytics?category=criminal", Value::Null)
            .await
            .1["metrics"]["overdue_follow_ups"],
        0
    );
    assert_eq!(
        demo.request(
            "PATCH",
            &path,
            json!({"expected_version":1,"action":"reopen","due_on":null})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        demo.request(
            "PATCH",
            &path,
            json!({"expected_version":2,"action":"reschedule","due_on":today})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        demo.request("GET", "/analytics?category=criminal", Value::Null)
            .await
            .1["metrics"]["overdue_follow_ups"],
        0
    );
    let detail = demo.detail(first).await;
    for event in [
        "follow_up.created",
        "follow_up.completed",
        "follow_up.reopened",
        "follow_up.rescheduled",
    ] {
        assert!(
            detail["activity"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["event_type"] == event)
        );
    }
    assert_eq!(
        demo.request("GET", "/analytics?category=invalid", Value::Null)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let foreign = Demo::new().await;
    assert_eq!(
        foreign
            .request("GET", "/analytics?category=criminal", Value::Null)
            .await
            .1["metrics"]["visible_matters"],
        0
    );
    assert_eq!(
        foreign
            .request(
                "PATCH",
                &format!("/matters/{first}/follow-ups/{task}"),
                json!({"expected_version":0,"action":"complete","due_on":null})
            )
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    // Assign this task to another staff member: own-matter access is not task-edit permission.
    sqlx::query("UPDATE matterflow.follow_ups SET assigned_to=(SELECT id FROM matterflow.app_users WHERE workspace_id=$1 AND role='staff' AND display_name LIKE 'Sam%') WHERE workspace_id=$1 AND id=$2::uuid").bind(demo.workspace).bind(task).execute(&demo.pool).await.unwrap();
    assert_eq!(
        demo.request(
            "PATCH",
            &format!("/matters/{first}/follow-ups/{task}"),
            json!({"expected_version":0,"action":"complete","due_on":null})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    demo.role("reviewer").await;
    assert_eq!(
        demo.request(
            "PATCH",
            &format!("/matters/{first}/follow-ups/{task}"),
            json!({"expected_version":0,"action":"complete","due_on":null})
        )
        .await
        .0,
        StatusCode::OK
    );
    foreign.finish().await;
    demo.finish().await;
}

#[tokio::test]
#[ignore = "requires migrated and seeded development database; run npm.cmd run test:analytics"]
async fn staff_and_reviewer_analytics_have_distinct_scopes_and_empty_denominators_are_null() {
    let demo = Demo::new().await;
    assert_eq!(
        demo.request("GET", "/analytics", Value::Null).await.1["metrics"]["visible_matters"],
        6
    );
    let empty = demo
        .request("GET", "/analytics?category=criminal", Value::Null)
        .await;
    assert_eq!(empty.1["metrics"]["visible_matters"], 0);
    assert!(empty.1["metrics"]["completeness_percent"].is_null());
    assert!(empty.1["metrics"]["median_days_to_readiness"].is_null());
    demo.role("reviewer").await;
    assert_eq!(
        demo.request("GET", "/analytics", Value::Null).await.1["metrics"]["visible_matters"],
        12
    );
    demo.finish().await;
}
