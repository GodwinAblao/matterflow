use super::session::{Role, Session, new_token, token_hash};
use crate::error::ApiError;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Serialize)]
pub struct CreatedSession {
    pub token: String,
    pub session: Session,
}

pub async fn create_demo(pool: &PgPool, role: Role) -> Result<CreatedSession, ApiError> {
    let token = new_token()?;
    let mut tx = pool.begin().await.map_err(ApiError::database)?;
    // Serialize demo creation to enforce the cap even under concurrent requests.
    sqlx::query("SELECT pg_advisory_xact_lock(74621002)")
        .execute(&mut *tx)
        .await
        .map_err(ApiError::database)?;
    sqlx::query(
        "DELETE FROM matterflow.workspaces WHERE id IN (
        SELECT id FROM matterflow.workspaces WHERE NOT is_template AND expires_at <= now() LIMIT 30
    )",
    )
    .execute(&mut *tx)
    .await
    .map_err(ApiError::database)?;
    let active: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM matterflow.workspaces WHERE NOT is_template AND expires_at > now()",
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::database)?;
    if active >= 30 {
        return Err(ApiError::capacity());
    }
    let (template, template_created_at): (Uuid, DateTime<Utc>) = sqlx::query_as(
        "SELECT id, created_at FROM matterflow.workspaces WHERE template_key = 'portfolio-v1' AND is_template",
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::database)?
    .ok_or_else(ApiError::unavailable)?;
    let (workspace, expires_at): (Uuid, DateTime<Utc>) = sqlx::query_as(
        "INSERT INTO matterflow.workspaces(name, expires_at) VALUES ('Private fictional demo', now() + interval '2 hours') RETURNING id, expires_at",
    ).fetch_one(&mut *tx).await.map_err(ApiError::database)?;
    // New deterministic IDs within a random workspace preserve relationships
    // while ensuring no copied row retains a template or visitor identifier.
    for statement in [
        "INSERT INTO matterflow.app_users(id, workspace_id, display_name, role, created_at)
         SELECT md5($1::uuid::text || id::text)::uuid, $1, display_name, role, created_at FROM matterflow.app_users WHERE workspace_id = $2",
        "INSERT INTO matterflow.matters(id, workspace_id, reference, title, client_label, service_category, status, assigned_staff_id, assigned_reviewer_id, created_at, updated_at, first_ready_at, closed_at)
         SELECT md5($1::uuid::text || id::text)::uuid, $1, reference, title, client_label, service_category, status,
            md5($1::uuid::text || assigned_staff_id::text)::uuid, md5($1::uuid::text || assigned_reviewer_id::text)::uuid,
            created_at, updated_at, first_ready_at, closed_at FROM matterflow.matters WHERE workspace_id = $2",
        "INSERT INTO matterflow.document_requirements(id, workspace_id, matter_id, title, is_required, status, requested_at, received_at, reviewed_at, reviewed_by, correction_note)
         SELECT md5($1::uuid::text || id::text)::uuid, $1, md5($1::uuid::text || matter_id::text)::uuid, title, is_required, status, requested_at, received_at, reviewed_at,
            md5($1::uuid::text || reviewed_by::text)::uuid, correction_note FROM matterflow.document_requirements WHERE workspace_id = $2",
        "INSERT INTO matterflow.follow_ups(id, workspace_id, matter_id, requirement_id, assigned_to, title, due_on, created_at, completed_at)
         SELECT md5($1::uuid::text || id::text)::uuid, $1, md5($1::uuid::text || matter_id::text)::uuid,
            md5($1::uuid::text || requirement_id::text)::uuid, md5($1::uuid::text || assigned_to::text)::uuid, title, due_on, created_at, completed_at
         FROM matterflow.follow_ups WHERE workspace_id = $2",
        "INSERT INTO matterflow.activity_events(id, workspace_id, matter_id, actor_id, event_type, details, occurred_at)
         SELECT md5($1::uuid::text || id::text)::uuid, $1, md5($1::uuid::text || matter_id::text)::uuid,
            md5($1::uuid::text || actor_id::text)::uuid, event_type, details, occurred_at FROM matterflow.activity_events WHERE workspace_id = $2",
    ] {
        sqlx::query(statement).bind(workspace).bind(template).execute(&mut *tx).await.map_err(ApiError::database)?;
    }
    // Keep sample ages and follow-up dates relative to each new visit.
    for statement in [
        "UPDATE matterflow.app_users SET created_at = created_at + (now() - $2::timestamptz) WHERE workspace_id = $1",
        "UPDATE matterflow.matters SET created_at = created_at + (now() - $2::timestamptz), updated_at = updated_at + (now() - $2::timestamptz), first_ready_at = first_ready_at + (now() - $2::timestamptz), closed_at = closed_at + (now() - $2::timestamptz) WHERE workspace_id = $1",
        "UPDATE matterflow.document_requirements SET requested_at = requested_at + (now() - $2::timestamptz), received_at = received_at + (now() - $2::timestamptz), reviewed_at = reviewed_at + (now() - $2::timestamptz) WHERE workspace_id = $1",
        "UPDATE matterflow.follow_ups SET due_on = due_on + ((now() AT TIME ZONE 'Asia/Manila')::date - ($2::timestamptz AT TIME ZONE 'Asia/Manila')::date), created_at = created_at + (now() - $2::timestamptz), completed_at = completed_at + (now() - $2::timestamptz) WHERE workspace_id = $1",
        "UPDATE matterflow.activity_events SET occurred_at = occurred_at + (now() - $2::timestamptz) WHERE workspace_id = $1",
    ] {
        sqlx::query(statement)
            .bind(workspace)
            .bind(template_created_at)
            .execute(&mut *tx)
            .await
            .map_err(ApiError::database)?;
    }
    let user: Uuid = sqlx::query_scalar("SELECT id FROM matterflow.app_users WHERE workspace_id = $1 AND role = $2 ORDER BY display_name LIMIT 1")
        .bind(workspace).bind(role.as_str()).fetch_one(&mut *tx).await.map_err(ApiError::database)?;
    let id: Uuid = sqlx::query_scalar("INSERT INTO matterflow.demo_sessions(workspace_id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4) RETURNING id")
        .bind(workspace).bind(user).bind(token_hash(&token)).bind(expires_at).fetch_one(&mut *tx).await.map_err(ApiError::database)?;
    let session = sqlx::query_as::<_, Session>("SELECT s.id, s.workspace_id, s.user_id, u.display_name, u.role, s.expires_at FROM matterflow.demo_sessions s JOIN matterflow.app_users u ON u.id = s.user_id WHERE s.id = $1")
        .bind(id).fetch_one(&mut *tx).await.map_err(ApiError::database)?;
    tx.commit().await.map_err(ApiError::database)?;
    Ok(CreatedSession { token, session })
}
