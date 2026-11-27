use sqlx::{Acquire, Postgres, Transaction};

// Explicit opt-in: tests use the configured development database, insert fixture
// rows inside a transaction, and roll back every fixture before returning.
#[tokio::test]
#[ignore = "requires a migrated development database; run npm.cmd run db:test"]
async fn schema_enforces_integrity_and_workspace_boundaries() {
    dotenvy::from_filename("backend/.env").ok();
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be configured");
    let pool = matterflow_api::db::create_pool(Some(&url))
        .unwrap()
        .unwrap();
    let mut tx = pool.begin().await.expect("connect to development database");
    let workspace: String = sqlx::query_scalar(
        "INSERT INTO matterflow.workspaces(name) VALUES ('Transactional schema test') RETURNING id::text",
    ).fetch_one(&mut *tx).await.unwrap();
    let other_workspace: String = sqlx::query_scalar(
        "INSERT INTO matterflow.workspaces(name) VALUES ('Other test workspace') RETURNING id::text",
    ).fetch_one(&mut *tx).await.unwrap();
    let reviewer: String = sqlx::query_scalar(
        "INSERT INTO matterflow.app_users(workspace_id, display_name, role) VALUES ($1::uuid, 'Test reviewer', 'reviewer') RETURNING id::text",
    ).bind(&workspace).fetch_one(&mut *tx).await.unwrap();
    let other_user: String = sqlx::query_scalar(
        "INSERT INTO matterflow.app_users(workspace_id, display_name, role) VALUES ($1::uuid, 'Other staff', 'staff') RETURNING id::text",
    ).bind(&other_workspace).fetch_one(&mut *tx).await.unwrap();
    let matter: String = sqlx::query_scalar(
        "INSERT INTO matterflow.matters(workspace_id, reference, title, client_label, service_category) VALUES ($1::uuid, 'TEST-001', 'Schema test matter', 'Fictional client', 'property') RETURNING id::text",
    ).bind(&workspace).fetch_one(&mut *tx).await.unwrap();
    let requirement: String = sqlx::query_scalar(
        "INSERT INTO matterflow.document_requirements(workspace_id, matter_id, title) VALUES ($1::uuid, $2::uuid, 'Test requirement') RETURNING id::text",
    ).bind(&workspace).bind(&matter).fetch_one(&mut *tx).await.unwrap();

    reject(
        &mut tx,
        &format!(
            "UPDATE matterflow.matters SET assigned_staff_id = '{other_user}' WHERE id = '{matter}'"
        ),
        "23503",
    )
    .await;
    reject(&mut tx, &format!("INSERT INTO matterflow.app_users(workspace_id, display_name, role) VALUES ('{workspace}', 'Invalid role', 'admin')"), "23514").await;
    reject(&mut tx, &format!("UPDATE matterflow.document_requirements SET status = 'accepted' WHERE id = '{requirement}'"), "23514").await;
    reject(
        &mut tx,
        &format!("UPDATE matterflow.matters SET status = 'ready' WHERE id = '{matter}'"),
        "23514",
    )
    .await;
    reject(&mut tx, &format!("INSERT INTO matterflow.matters(workspace_id, reference, title, client_label, service_category) VALUES ('{workspace}', 'TEST-001', 'Duplicate reference', 'Fictional client', 'property')"), "23505").await;

    // A properly recorded review is accepted by the schema.
    sqlx::query("UPDATE matterflow.document_requirements SET status = 'accepted', received_at = now(), reviewed_at = now(), reviewed_by = $1::uuid WHERE id = $2::uuid")
        .bind(&reviewer).bind(&requirement).execute(&mut *tx).await.unwrap();

    // Demonstrate that a failed activity write rolls back its related status change.
    let mut change = tx.begin().await.unwrap();
    sqlx::query("UPDATE matterflow.matters SET status = 'under_review' WHERE id = $1::uuid")
        .bind(&matter)
        .execute(&mut *change)
        .await
        .unwrap();
    let result = sqlx::query("INSERT INTO matterflow.activity_events(workspace_id, matter_id, actor_id, event_type) VALUES ($1::uuid, $2::uuid, $3::uuid, 'test.changed')")
        .bind(&workspace).bind(&matter).bind(&other_user).execute(&mut *change).await;
    assert!(result.is_err());
    change.rollback().await.unwrap();
    let status: String =
        sqlx::query_scalar("SELECT status FROM matterflow.matters WHERE id = $1::uuid")
            .bind(&matter)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(status, "intake");
    tx.rollback()
        .await
        .expect("remove all transactional fixtures");
    pool.close().await;
}

async fn reject(tx: &mut Transaction<'_, Postgres>, statement: &str, expected_code: &str) {
    let mut savepoint = tx.begin().await.unwrap();
    let error = sqlx::query(statement)
        .execute(&mut *savepoint)
        .await
        .expect_err("invalid data should be rejected");
    let code = error.as_database_error().and_then(|error| error.code());
    assert_eq!(code.as_deref(), Some(expected_code));
    savepoint.rollback().await.unwrap();
}
