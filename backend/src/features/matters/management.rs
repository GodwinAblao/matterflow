use crate::{
    error::ApiError,
    features::{
        auth::{Session, session::pool},
        matters::{creation::Category, workflow},
    },
    state::AppState,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

pub const DELETABLE: &str = "m.creation_key IS NOT NULL AND m.status='intake' AND NOT EXISTS(SELECT 1 FROM matterflow.document_requirements d WHERE d.workspace_id=m.workspace_id AND d.matter_id=m.id AND (d.status<>'missing' OR d.version<>0)) AND NOT EXISTS(SELECT 1 FROM matterflow.follow_ups f WHERE f.workspace_id=m.workspace_id AND f.matter_id=m.id) AND NOT EXISTS(SELECT 1 FROM matterflow.activity_events e WHERE e.workspace_id=m.workspace_id AND e.matter_id=m.id AND e.event_type NOT IN ('matter.opened','matter.updated'))";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateInput {
    pub expected_version: i32,
    pub title: String,
    pub client_label: String,
    pub service_category: Category,
    pub assigned_staff_id: Uuid,
}
pub async fn update(
    session: Session,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateInput>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let title = input.title.trim();
    let client = input.client_label.trim();
    if title.is_empty()
        || title.chars().count() > 200
        || client.is_empty()
        || client.chars().count() > 120
    {
        return Err(ApiError::validation(
            "Provide a title of 1–200 characters and a fictional client label of 1–120 characters.",
        ));
    }
    let mut tx = pool(&state)?.begin().await.map_err(ApiError::database)?;
    let actor = workflow::current_actor(&mut tx, &session).await?;
    let locked = workflow::lock_matter(&mut tx, &actor, id).await?;
    workflow::check_version(locked.version, input.expected_version)?;
    if matches!(locked.status.as_str(), "ready" | "closed") {
        return Err(ApiError::conflict(
            "matter_locked",
            "Ready or closed matters cannot be edited.",
        ));
    }
    if actor.role == "staff" && input.assigned_staff_id != actor.user_id {
        return Err(ApiError::forbidden());
    }
    let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM matterflow.app_users WHERE workspace_id=$1 AND id=$2 AND role='staff')").bind(actor.workspace_id).bind(input.assigned_staff_id).fetch_one(&mut *tx).await.map_err(ApiError::database)?;
    if !valid {
        return Err(ApiError::validation(
            "Choose a staff member from this demo.",
        ));
    }
    let before:serde_json::Value=sqlx::query_scalar("SELECT jsonb_build_object('title',title,'client_label',client_label,'service_category',service_category,'assigned_staff_id',assigned_staff_id) FROM matterflow.matters WHERE workspace_id=$1 AND id=$2").bind(actor.workspace_id).bind(id).fetch_one(&mut *tx).await.map_err(ApiError::database)?;
    // Carry open tasks with the assignment so the new staff member can manage them.
    sqlx::query("UPDATE matterflow.follow_ups SET assigned_to=$3,version=version+1 WHERE workspace_id=$1 AND matter_id=$2 AND completed_at IS NULL AND assigned_to IS DISTINCT FROM $3").bind(actor.workspace_id).bind(id).bind(input.assigned_staff_id).execute(&mut *tx).await.map_err(ApiError::database)?;
    sqlx::query("UPDATE matterflow.matters SET title=$3,client_label=$4,service_category=$5,assigned_staff_id=$6,updated_at=clock_timestamp(),version=version+1 WHERE workspace_id=$1 AND id=$2").bind(actor.workspace_id).bind(id).bind(title).bind(client).bind(input.service_category.as_str()).bind(input.assigned_staff_id).execute(&mut *tx).await.map_err(ApiError::database)?;
    workflow::write_event(&mut tx,&actor,id,"matter.updated",json!({"before":before,"after":{"title":title,"client_label":client,"service_category":input.service_category,"assigned_staff_id":input.assigned_staff_id}})).await?;
    tx.commit().await.map_err(ApiError::database)?;
    Ok(Json(json!({"id":id})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmInput {
    pub expected_version: i32,
    pub reference: String,
}

pub async fn delete(
    session: Session,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<ConfirmInput>,
) -> Result<StatusCode, ApiError> {
    let mut tx = pool(&state)?.begin().await.map_err(ApiError::database)?;
    let actor = workflow::current_actor(&mut tx, &session).await?;
    let locked = workflow::lock_matter(&mut tx, &actor, id).await?;
    workflow::check_version(locked.version, input.expected_version)?;
    let query = format!(
        "SELECT ({DELETABLE}) AND reference=$3 FROM matterflow.matters m WHERE workspace_id=$1 AND id=$2"
    );
    let allowed: bool = sqlx::query_scalar(&query)
        .bind(actor.workspace_id)
        .bind(id)
        .bind(input.reference.trim())
        .fetch_one(&mut *tx)
        .await
        .map_err(ApiError::database)?;
    if !allowed {
        return Err(ApiError::conflict(
            "delete_not_allowed",
            "Only a new intake matter without workflow activity can be deleted. Confirm its exact reference, or ask a reviewer to archive it.",
        ));
    }
    workflow::write_event(
        &mut tx,
        &actor,
        id,
        "matter.deleted",
        json!({"reference":input.reference}),
    )
    .await?;
    sqlx::query("UPDATE matterflow.matters SET deleted_at=clock_timestamp(),updated_at=clock_timestamp(),version=version+1 WHERE workspace_id=$1 AND id=$2").bind(actor.workspace_id).bind(id).execute(&mut *tx).await.map_err(ApiError::database)?;
    tx.commit().await.map_err(ApiError::database)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn archive(
    session: Session,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<ConfirmInput>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut tx = pool(&state)?.begin().await.map_err(ApiError::database)?;
    let actor = workflow::current_actor(&mut tx, &session).await?;
    if actor.role != "reviewer" {
        return Err(ApiError::forbidden());
    }
    let locked = workflow::lock_matter(&mut tx, &actor, id).await?;
    workflow::check_version(locked.version, input.expected_version)?;
    let reference: String = sqlx::query_scalar(
        "SELECT reference FROM matterflow.matters WHERE workspace_id=$1 AND id=$2",
    )
    .bind(actor.workspace_id)
    .bind(id)
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::database)?;
    if reference != input.reference.trim() || locked.status == "closed" {
        return Err(ApiError::conflict(
            "archive_not_allowed",
            "Confirm the exact reference of an open matter.",
        ));
    }
    sqlx::query("UPDATE matterflow.matters SET status='closed',closed_at=clock_timestamp(),updated_at=clock_timestamp(),version=version+1 WHERE workspace_id=$1 AND id=$2").bind(actor.workspace_id).bind(id).execute(&mut *tx).await.map_err(ApiError::database)?;
    workflow::write_event(
        &mut tx,
        &actor,
        id,
        "matter.archived",
        json!({"from_status":locked.status,"to_status":"closed"}),
    )
    .await?;
    tx.commit().await.map_err(ApiError::database)?;
    Ok(Json(json!({"id":id})))
}
