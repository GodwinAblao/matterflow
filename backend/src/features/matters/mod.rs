use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    error::ApiError,
    features::auth::{Session, session::pool},
    state::AppState,
};

#[derive(FromRow, Serialize)]
pub struct MatterSummary {
    pub id: Uuid,
    pub reference: String,
    pub title: String,
    pub client_label: String,
    pub service_category: String,
    pub status: String,
    pub assigned_to: Option<String>,
    pub required_documents: i64,
    pub accepted_documents: i64,
}

const SUMMARY: &str = "SELECT m.id, m.reference, m.title, m.client_label, m.service_category, m.status,
    u.display_name AS assigned_to,
    (SELECT count(*) FROM matterflow.document_requirements d WHERE d.workspace_id = m.workspace_id AND d.matter_id = m.id AND d.is_required) AS required_documents,
    (SELECT count(*) FROM matterflow.document_requirements d WHERE d.workspace_id = m.workspace_id AND d.matter_id = m.id AND d.is_required AND d.status = 'accepted') AS accepted_documents
    FROM matterflow.matters m LEFT JOIN matterflow.app_users u ON (u.workspace_id, u.id) = (m.workspace_id, m.assigned_staff_id)
    WHERE m.workspace_id = $1 AND ($2 = 'reviewer' OR m.assigned_staff_id = $3)";

pub async fn list(
    session: Session,
    State(state): State<AppState>,
) -> Result<Json<Vec<MatterSummary>>, ApiError> {
    let query = format!("{SUMMARY} ORDER BY m.reference LIMIT 100");
    let matters = sqlx::query_as::<_, MatterSummary>(&query)
        .bind(session.workspace_id)
        .bind(&session.role)
        .bind(session.user_id)
        .fetch_all(pool(&state)?)
        .await
        .map_err(ApiError::database)?;
    Ok(Json(matters))
}

#[derive(FromRow, Serialize)]
pub struct Requirement {
    pub id: Uuid,
    pub title: String,
    pub is_required: bool,
    pub status: String,
    pub correction_note: Option<String>,
}

#[derive(FromRow, Serialize)]
pub struct FollowUp {
    pub id: Uuid,
    pub title: String,
    pub due_on: NaiveDate,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct MatterDetail {
    pub matter: MatterSummary,
    pub documents: Vec<Requirement>,
    pub follow_ups: Vec<FollowUp>,
}

pub async fn detail(
    session: Session,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<MatterDetail>, ApiError> {
    let pool = pool(&state)?;
    let query = format!("{SUMMARY} AND m.id = $4");
    let matter = sqlx::query_as::<_, MatterSummary>(&query)
        .bind(session.workspace_id)
        .bind(&session.role)
        .bind(session.user_id)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(ApiError::database)?
        .ok_or_else(ApiError::not_found)?;
    let documents = sqlx::query_as::<_, Requirement>("SELECT id, title, is_required, status, correction_note FROM matterflow.document_requirements WHERE workspace_id = $1 AND matter_id = $2 ORDER BY title")
        .bind(session.workspace_id).bind(id).fetch_all(pool).await.map_err(ApiError::database)?;
    let follow_ups = sqlx::query_as::<_, FollowUp>("SELECT id, title, due_on, completed_at FROM matterflow.follow_ups WHERE workspace_id = $1 AND matter_id = $2 ORDER BY due_on, id")
        .bind(session.workspace_id).bind(id).fetch_all(pool).await.map_err(ApiError::database)?;
    Ok(Json(MatterDetail {
        matter,
        documents,
        follow_ups,
    }))
}
