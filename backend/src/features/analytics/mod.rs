use crate::{
    error::ApiError,
    features::auth::{Session, session::pool},
    state::AppState,
};
use axum::{
    Json,
    extract::{Query, State},
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filters {
    pub category: Option<String>,
}
#[derive(FromRow, Serialize)]
pub struct Matter {
    pub id: Uuid,
    pub reference: String,
    pub title: String,
    pub status: String,
    pub service_category: String,
    pub assigned_to: Option<String>,
    pub required_documents: i64,
    pub accepted_documents: i64,
    pub days_to_readiness: Option<f64>,
}
#[derive(FromRow, Serialize)]
pub struct Requirement {
    pub id: Uuid,
    pub matter_id: Uuid,
    pub reference: String,
    pub title: String,
    pub status: String,
    pub age_days: i32,
}
#[derive(FromRow, Serialize)]
pub struct FollowUp {
    pub id: Uuid,
    pub matter_id: Uuid,
    pub reference: String,
    pub title: String,
    pub due_on: NaiveDate,
    pub assigned_to: Option<String>,
    pub completed: bool,
    pub matter_status: String,
    pub overdue: bool,
}
#[derive(Serialize)]
pub struct Metrics {
    pub visible_matters: usize,
    pub active_matters: usize,
    pub required_documents: i64,
    pub accepted_documents: i64,
    pub completeness_percent: Option<f64>,
    pub outstanding_requirements: usize,
    pub overdue_follow_ups: usize,
    pub median_days_to_readiness: Option<f64>,
    pub readiness_cohort: usize,
}
#[derive(Serialize)]
pub struct Report {
    pub as_of: DateTime<Utc>,
    pub local_date: NaiveDate,
    pub metrics: Metrics,
    pub matters: Vec<Matter>,
    pub requirements: Vec<Requirement>,
    pub follow_ups: Vec<FollowUp>,
}

pub fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    Some(if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    })
}

pub async fn report(
    session: Session,
    State(state): State<AppState>,
    Query(filters): Query<Filters>,
) -> Result<Json<Report>, ApiError> {
    if filters.category.as_deref().is_some_and(|category| {
        !matches!(
            category,
            "property" | "business" | "family" | "criminal" | "other"
        )
    }) {
        return Err(ApiError::validation("Choose a valid service category."));
    }
    let mut tx = pool(&state)?.begin().await.map_err(ApiError::database)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(ApiError::database)?;
    let (as_of, local_date): (DateTime<Utc>, NaiveDate) =
        sqlx::query_as("SELECT now(),(now() AT TIME ZONE 'Asia/Manila')::date")
            .fetch_one(&mut *tx)
            .await
            .map_err(ApiError::database)?;
    let matters=sqlx::query_as::<_,Matter>("SELECT m.id,m.reference,m.title,m.status,m.service_category,u.display_name AS assigned_to,(SELECT count(*) FROM matterflow.document_requirements d WHERE d.workspace_id=m.workspace_id AND d.matter_id=m.id AND d.is_required) AS required_documents,(SELECT count(*) FROM matterflow.document_requirements d WHERE d.workspace_id=m.workspace_id AND d.matter_id=m.id AND d.is_required AND d.status='accepted') AS accepted_documents,EXTRACT(EPOCH FROM (m.first_ready_at-m.created_at))::double precision/86400.0 AS days_to_readiness FROM matterflow.matters m LEFT JOIN matterflow.app_users u ON (u.workspace_id,u.id)=(m.workspace_id,m.assigned_staff_id) WHERE m.deleted_at IS NULL AND m.workspace_id=$1 AND ($2='reviewer' OR m.assigned_staff_id=$3) AND ($4::text IS NULL OR m.service_category=$4) ORDER BY m.reference")
        .bind(session.workspace_id).bind(&session.role).bind(session.user_id).bind(&filters.category).fetch_all(&mut *tx).await.map_err(ApiError::database)?;
    let requirements=sqlx::query_as::<_,Requirement>("SELECT d.id,d.matter_id,m.reference,d.title,d.status,GREATEST(0,FLOOR(EXTRACT(EPOCH FROM(now()-d.requested_at))/86400))::int AS age_days FROM matterflow.document_requirements d JOIN matterflow.matters m ON (m.workspace_id,m.id)=(d.workspace_id,d.matter_id) WHERE m.deleted_at IS NULL AND m.workspace_id=$1 AND ($2='reviewer' OR m.assigned_staff_id=$3) AND ($4::text IS NULL OR m.service_category=$4) AND m.status<>'closed' AND d.is_required AND d.status<>'accepted' ORDER BY age_days DESC,m.reference,d.title")
        .bind(session.workspace_id).bind(&session.role).bind(session.user_id).bind(&filters.category).fetch_all(&mut *tx).await.map_err(ApiError::database)?;
    let follow_ups=sqlx::query_as::<_,FollowUp>("SELECT f.id,f.matter_id,m.reference,f.title,f.due_on,u.display_name AS assigned_to,f.completed_at IS NOT NULL AS completed,m.status AS matter_status,(f.completed_at IS NULL AND f.due_on<$5 AND m.status<>'closed') AS overdue FROM matterflow.follow_ups f JOIN matterflow.matters m ON (m.workspace_id,m.id)=(f.workspace_id,f.matter_id) LEFT JOIN matterflow.app_users u ON (u.workspace_id,u.id)=(f.workspace_id,f.assigned_to) WHERE m.deleted_at IS NULL AND m.workspace_id=$1 AND ($2='reviewer' OR(m.assigned_staff_id=$3 AND f.assigned_to=$3)) AND ($4::text IS NULL OR m.service_category=$4) ORDER BY f.due_on,m.reference,f.id")
        .bind(session.workspace_id).bind(&session.role).bind(session.user_id).bind(&filters.category).bind(local_date).fetch_all(&mut *tx).await.map_err(ApiError::database)?;
    tx.commit().await.map_err(ApiError::database)?;
    let required = matters
        .iter()
        .map(|matter| matter.required_documents)
        .sum::<i64>();
    let accepted = matters
        .iter()
        .map(|matter| matter.accepted_documents)
        .sum::<i64>();
    let cohort = matters
        .iter()
        .filter_map(|matter| matter.days_to_readiness)
        .collect::<Vec<_>>();
    let metrics = Metrics {
        visible_matters: matters.len(),
        active_matters: matters
            .iter()
            .filter(|matter| matter.status != "closed")
            .count(),
        required_documents: required,
        accepted_documents: accepted,
        completeness_percent: if required > 0 {
            Some(100.0 * accepted as f64 / required as f64)
        } else {
            None
        },
        outstanding_requirements: requirements.len(),
        overdue_follow_ups: follow_ups
            .iter()
            .filter(|follow_up| follow_up.overdue)
            .count(),
        readiness_cohort: cohort.len(),
        median_days_to_readiness: median(cohort),
    };
    Ok(Json(Report {
        as_of,
        local_date,
        metrics,
        matters,
        requirements,
        follow_ups,
    }))
}

#[cfg(test)]
mod tests {
    use super::median;
    #[test]
    fn readiness_median_handles_empty_odd_and_even_cohorts() {
        assert_eq!(median(vec![]), None);
        assert_eq!(median(vec![9.0, 1.0, 3.0]), Some(3.0));
        assert_eq!(median(vec![9.0, 1.0, 3.0, 5.0]), Some(4.0));
    }
}
