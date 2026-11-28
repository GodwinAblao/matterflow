use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::{
    service::CreatedSession,
    session::{Role, Session, pool},
};
use crate::{error::ApiError, state::AppState};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChooseRole {
    pub role: Role,
}

pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<ChooseRole>,
) -> Result<(StatusCode, Json<CreatedSession>), ApiError> {
    let created = super::service::create_demo(pool(&state)?, input.role).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn current(session: Session) -> Json<Session> {
    Json(session)
}

pub async fn switch_role(
    session: Session,
    State(state): State<AppState>,
    Json(input): Json<ChooseRole>,
) -> Result<Json<Session>, ApiError> {
    let session = sqlx::query_as::<_, Session>(
        "UPDATE matterflow.demo_sessions s SET user_id = u.id FROM matterflow.app_users u
         WHERE s.id = $1 AND s.expires_at > now() AND u.workspace_id = s.workspace_id
           AND u.id = (SELECT id FROM matterflow.app_users WHERE workspace_id = s.workspace_id AND role = $2 ORDER BY display_name LIMIT 1)
         RETURNING s.id, s.workspace_id, s.user_id, u.display_name, u.role, s.expires_at",
    ).bind(session.id).bind(input.role.as_str()).fetch_optional(pool(&state)?).await
        .map_err(ApiError::database)?.ok_or_else(ApiError::unauthorized)?;
    Ok(Json(session))
}

pub async fn end(session: Session, State(state): State<AppState>) -> Result<StatusCode, ApiError> {
    sqlx::query("DELETE FROM matterflow.workspaces WHERE id = $1 AND NOT is_template")
        .bind(session.workspace_id)
        .execute(pool(&state)?)
        .await
        .map_err(ApiError::database)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(FromRow, Serialize)]
pub struct TeamMember {
    pub display_name: String,
    pub role: String,
    pub active_matters: i64,
}

pub async fn team(
    session: Session,
    State(state): State<AppState>,
) -> Result<Json<Vec<TeamMember>>, ApiError> {
    session.require_reviewer()?;
    let team = sqlx::query_as::<_, TeamMember>(
        "SELECT u.display_name, u.role, count(m.id) AS active_matters
         FROM matterflow.app_users u LEFT JOIN matterflow.matters m
           ON m.workspace_id = u.workspace_id AND m.assigned_staff_id = u.id AND m.status <> 'closed' AND m.deleted_at IS NULL
         WHERE u.workspace_id = $1 GROUP BY u.id ORDER BY u.role DESC, u.display_name",
    ).bind(session.workspace_id).fetch_all(pool(&state)?).await.map_err(ApiError::database)?;
    Ok(Json(team))
}
