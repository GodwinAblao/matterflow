use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{error::ApiError, features::auth::Session};

#[derive(FromRow)]
pub struct LockedMatter {
    pub status: String,
    pub version: i32,
}

#[derive(FromRow, Serialize)]
pub struct Blocker {
    pub requirement_id: Option<Uuid>,
    pub title: String,
    pub status: String,
    pub reason: String,
}

#[derive(Serialize)]
pub struct Readiness {
    pub documents_complete: bool,
    pub blockers: Vec<Blocker>,
}

#[derive(Serialize)]
pub struct ChangeResult {
    pub matter_id: Uuid,
    pub status: String,
    pub version: i32,
}

pub async fn current_actor(
    tx: &mut Transaction<'_, Postgres>,
    session: &Session,
) -> Result<Session, ApiError> {
    // Hold workspace/session locks until commit. Role switches and logout cannot
    // invalidate the authorization decision in the middle of a workflow change.
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM matterflow.workspaces WHERE id = $1 AND NOT is_template AND expires_at > clock_timestamp() FOR SHARE")
        .bind(session.workspace_id).fetch_optional(&mut **tx).await.map_err(ApiError::database)?.ok_or_else(ApiError::unauthorized)?;
    sqlx::query_as::<_, Session>("SELECT s.id, s.workspace_id, s.user_id, u.display_name, u.role, s.expires_at FROM matterflow.demo_sessions s JOIN matterflow.app_users u ON (u.workspace_id,u.id)=(s.workspace_id,s.user_id) WHERE s.id=$1 AND s.workspace_id=$2 AND s.expires_at > clock_timestamp() FOR SHARE OF s")
        .bind(session.id).bind(session.workspace_id).fetch_optional(&mut **tx).await.map_err(ApiError::database)?.ok_or_else(ApiError::unauthorized)
}

pub async fn lock_matter(
    tx: &mut Transaction<'_, Postgres>,
    actor: &Session,
    matter_id: Uuid,
) -> Result<LockedMatter, ApiError> {
    sqlx::query_as::<_, LockedMatter>("SELECT status, version FROM matterflow.matters WHERE workspace_id=$1 AND id=$2 AND ($3='reviewer' OR assigned_staff_id=$4) FOR UPDATE")
        .bind(actor.workspace_id).bind(matter_id).bind(&actor.role).bind(actor.user_id)
        .fetch_optional(&mut **tx).await.map_err(ApiError::database)?.ok_or_else(ApiError::not_found)
}

pub async fn readiness(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    matter_id: Uuid,
) -> Result<Readiness, ApiError> {
    let total: i64=sqlx::query_scalar("SELECT count(*) FROM matterflow.document_requirements WHERE workspace_id=$1 AND matter_id=$2 AND is_required")
        .bind(workspace_id).bind(matter_id).fetch_one(&mut **tx).await.map_err(ApiError::database)?;
    let mut blockers=sqlx::query_as::<_, Blocker>("SELECT id AS requirement_id, title, status, CASE status WHEN 'missing' THEN 'Document has not been received.' WHEN 'received' THEN 'Document is awaiting reviewer acceptance.' WHEN 'needs_correction' THEN 'A corrected submission is required.' ELSE 'Review is incomplete.' END AS reason FROM matterflow.document_requirements WHERE workspace_id=$1 AND matter_id=$2 AND is_required AND status <> 'accepted' ORDER BY title")
        .bind(workspace_id).bind(matter_id).fetch_all(&mut **tx).await.map_err(ApiError::database)?;
    if total == 0 {
        blockers.push(Blocker {
            requirement_id: None,
            title: "Required checklist".into(),
            status: "empty".into(),
            reason: "At least one required document must be defined.".into(),
        });
    }
    Ok(Readiness {
        documents_complete: blockers.is_empty(),
        blockers,
    })
}

pub async fn write_event(
    tx: &mut Transaction<'_, Postgres>,
    actor: &Session,
    matter_id: Uuid,
    event_type: &str,
    details: Value,
) -> Result<(), ApiError> {
    sqlx::query("INSERT INTO matterflow.activity_events(workspace_id,matter_id,actor_id,event_type,details,occurred_at) VALUES ($1,$2,$3,$4,$5,clock_timestamp())")
        .bind(actor.workspace_id).bind(matter_id).bind(actor.user_id).bind(event_type).bind(sqlx::types::Json(details))
        .execute(&mut **tx).await.map_err(ApiError::database)?;
    Ok(())
}

pub fn check_version(actual: i32, expected: i32) -> Result<(), ApiError> {
    if actual == expected {
        Ok(())
    } else {
        Err(ApiError::conflict(
            "stale_version",
            "This record changed. Review the latest information before trying again.",
        ))
    }
}

pub async fn mark_ready(
    pool: &PgPool,
    session: &Session,
    matter_id: Uuid,
    expected: i32,
) -> Result<ChangeResult, ApiError> {
    if expected < 0 {
        return Err(ApiError::validation("A valid record version is required."));
    }
    let mut tx = pool.begin().await.map_err(ApiError::database)?;
    let actor = current_actor(&mut tx, session).await?;
    actor.require_reviewer()?;
    let matter = lock_matter(&mut tx, &actor, matter_id).await?;
    if matter.status == "closed" {
        return Err(ApiError::conflict(
            "matter_closed",
            "A closed matter cannot be changed.",
        ));
    }
    if matter.status == "ready" {
        tx.commit().await.map_err(ApiError::database)?;
        return Ok(ChangeResult {
            matter_id,
            status: matter.status,
            version: matter.version,
        });
    }
    check_version(matter.version, expected)?;
    let check = readiness(&mut tx, actor.workspace_id, matter_id).await?;
    if !check.documents_complete {
        let mut error = ApiError::conflict(
            "readiness_blocked",
            "Resolve every required document before marking this matter ready.",
        );
        error.details = Some(json!({"blockers":check.blockers}));
        return Err(error);
    }
    let version:i32=sqlx::query_scalar("UPDATE matterflow.matters SET status='ready', first_ready_at=COALESCE(first_ready_at,clock_timestamp()), updated_at=clock_timestamp(),version=version+1 WHERE workspace_id=$1 AND id=$2 RETURNING version")
        .bind(actor.workspace_id).bind(matter_id).fetch_one(&mut *tx).await.map_err(ApiError::database)?;
    write_event(
        &mut tx,
        &actor,
        matter_id,
        "matter.ready",
        json!({"from_status":matter.status,"to_status":"ready","version":version}),
    )
    .await?;
    tx.commit().await.map_err(ApiError::database)?;
    Ok(ChangeResult {
        matter_id,
        status: "ready".into(),
        version,
    })
}

#[derive(FromRow, Serialize)]
pub struct Activity {
    pub id: Uuid,
    pub event_type: String,
    pub actor_name: Option<String>,
    pub details: Value,
    pub occurred_at: DateTime<Utc>,
}
