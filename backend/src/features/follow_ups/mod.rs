use crate::{
    error::ApiError,
    features::{
        auth::{Session, session::pool},
        matters::workflow,
    },
    state::AppState,
};
use axum::{
    Json,
    extract::{Path, State},
};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateInput {
    pub title: String,
    pub due_on: NaiveDate,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Complete,
    Reopen,
    Reschedule,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateInput {
    pub expected_version: i32,
    pub action: Action,
    pub due_on: Option<NaiveDate>,
}

pub async fn create(
    session: Session,
    State(state): State<AppState>,
    Path(matter): Path<Uuid>,
    Json(input): Json<CreateInput>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let title = input.title.trim();
    if title.is_empty() || title.chars().count() > 200 {
        return Err(ApiError::validation(
            "A follow-up title between 1 and 200 characters is required.",
        ));
    }
    let mut tx = pool(&state)?.begin().await.map_err(ApiError::database)?;
    let actor = workflow::current_actor(&mut tx, &session).await?;
    let locked = workflow::lock_matter(&mut tx, &actor, matter).await?;
    if locked.status == "closed" {
        return Err(ApiError::conflict(
            "matter_closed",
            "Follow-ups on closed matters cannot be changed.",
        ));
    }
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM matterflow.follow_ups WHERE workspace_id=$1 AND matter_id=$2",
    )
    .bind(actor.workspace_id)
    .bind(matter)
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::database)?;
    if count >= 50 {
        return Err(ApiError::validation(
            "A demo matter supports up to 50 follow-ups.",
        ));
    }
    let assigned: Option<Uuid> = sqlx::query_scalar(
        "SELECT assigned_staff_id FROM matterflow.matters WHERE workspace_id=$1 AND id=$2",
    )
    .bind(actor.workspace_id)
    .bind(matter)
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::database)?;
    let id:Uuid=sqlx::query_scalar("INSERT INTO matterflow.follow_ups(workspace_id,matter_id,assigned_to,title,due_on,created_at) VALUES($1,$2,$3,$4,$5,clock_timestamp()) RETURNING id")
        .bind(actor.workspace_id).bind(matter).bind(assigned).bind(title).bind(input.due_on).fetch_one(&mut *tx).await.map_err(ApiError::database)?;
    sqlx::query("UPDATE matterflow.matters SET updated_at=clock_timestamp(),version=version+1 WHERE workspace_id=$1 AND id=$2").bind(actor.workspace_id).bind(matter).execute(&mut *tx).await.map_err(ApiError::database)?;
    workflow::write_event(
        &mut tx,
        &actor,
        matter,
        "follow_up.created",
        json!({"follow_up_id":id,"follow_up_title":title,"due_on":input.due_on}),
    )
    .await?;
    tx.commit().await.map_err(ApiError::database)?;
    Ok(Json(json!({"id":id,"version":0})))
}

pub async fn update(
    session: Session,
    State(state): State<AppState>,
    Path((matter, id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateInput>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if input.expected_version < 0 {
        return Err(ApiError::validation(
            "A valid follow-up version is required.",
        ));
    }
    if matches!(input.action, Action::Reschedule) != input.due_on.is_some() {
        return Err(ApiError::validation(
            "Supply a due date only when rescheduling.",
        ));
    }
    let mut tx = pool(&state)?.begin().await.map_err(ApiError::database)?;
    let actor = workflow::current_actor(&mut tx, &session).await?;
    let locked = workflow::lock_matter(&mut tx, &actor, matter).await?;
    if locked.status == "closed" {
        return Err(ApiError::conflict(
            "matter_closed",
            "Follow-ups on closed matters cannot be changed.",
        ));
    }
    let (title,assigned,due,completed,version):(String,Option<Uuid>,NaiveDate,bool,i32)=sqlx::query_as("SELECT title,assigned_to,due_on,completed_at IS NOT NULL,version FROM matterflow.follow_ups WHERE workspace_id=$1 AND matter_id=$2 AND id=$3 FOR UPDATE")
        .bind(actor.workspace_id).bind(matter).bind(id).fetch_optional(&mut *tx).await.map_err(ApiError::database)?.ok_or_else(ApiError::not_found)?;
    if actor.role != "reviewer" && assigned != Some(actor.user_id) {
        return Err(ApiError::forbidden());
    }
    workflow::check_version(version, input.expected_version)?;
    let (event, target_completed, target_due) = match input.action {
        Action::Complete if !completed => ("follow_up.completed", true, due),
        Action::Reopen if completed => ("follow_up.reopened", false, due),
        Action::Reschedule if !completed => ("follow_up.rescheduled", false, input.due_on.unwrap()),
        _ => {
            return Err(ApiError::conflict(
                "invalid_transition",
                "Complete an open follow-up, or reopen it before rescheduling.",
            ));
        }
    };
    if matches!(input.action, Action::Reschedule) && target_due == due {
        return Err(ApiError::validation("Choose a different due date."));
    }
    sqlx::query("UPDATE matterflow.follow_ups SET due_on=$4,completed_at=CASE WHEN $5 THEN clock_timestamp() ELSE NULL END,version=version+1 WHERE workspace_id=$1 AND matter_id=$2 AND id=$3")
        .bind(actor.workspace_id).bind(matter).bind(id).bind(target_due).bind(target_completed).execute(&mut *tx).await.map_err(ApiError::database)?;
    sqlx::query("UPDATE matterflow.matters SET updated_at=clock_timestamp(),version=version+1 WHERE workspace_id=$1 AND id=$2").bind(actor.workspace_id).bind(matter).execute(&mut *tx).await.map_err(ApiError::database)?;
    workflow::write_event(&mut tx,&actor,matter,event,json!({"follow_up_id":id,"follow_up_title":title,"from_due_on":due,"due_on":target_due,"version":version+1})).await?;
    tx.commit().await.map_err(ApiError::database)?;
    Ok(Json(json!({"id":id,"version":version+1})))
}
