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
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Property,
    Business,
    Family,
    Criminal,
    Other,
}
impl Category {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Property => "property",
            Self::Business => "business",
            Self::Family => "family",
            Self::Criminal => "criminal",
            Self::Other => "other",
        }
    }
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequirementInput {
    pub title: String,
    pub is_required: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateInput {
    pub creation_key: Uuid,
    pub title: String,
    pub client_label: String,
    pub service_category: Category,
    pub assigned_staff_id: Option<Uuid>,
    pub documents: Vec<RequirementInput>,
}
#[derive(Serialize)]
pub struct Created {
    pub id: Uuid,
    pub reference: String,
    pub replayed: bool,
}

pub async fn create(
    session: Session,
    State(state): State<AppState>,
    Json(mut input): Json<CreateInput>,
) -> Result<(StatusCode, Json<Created>), ApiError> {
    input.title = input.title.trim().to_owned();
    input.client_label = input.client_label.trim().to_owned();
    if input.title.is_empty() || input.title.chars().count() > 200 {
        return Err(ApiError::validation(
            "A matter title between 1 and 200 characters is required.",
        ));
    }
    if input.client_label.is_empty() || input.client_label.chars().count() > 120 {
        return Err(ApiError::validation(
            "A fictional client label between 1 and 120 characters is required.",
        ));
    }
    if input.documents.is_empty()
        || input.documents.len() > 20
        || !input.documents.iter().any(|document| document.is_required)
    {
        return Err(ApiError::validation(
            "Provide 1 to 20 checklist items, including at least one required document.",
        ));
    }
    let mut titles = std::collections::BTreeSet::new();
    for document in &mut input.documents {
        document.title = document
            .title
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if document.title.is_empty() || document.title.chars().count() > 200 {
            return Err(ApiError::validation(
                "Each checklist title must contain 1 to 200 characters.",
            ));
        }
        if !titles.insert(document.title.to_lowercase()) {
            return Err(ApiError::validation("Checklist titles must be unique."));
        }
    }
    let mut tx = pool(&state)?.begin().await.map_err(ApiError::database)?;
    // Serialize only creation requests in this workspace, without upgrading the
    // shared authorization row lock used by other workflow writes.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text,74621003))")
        .bind(session.workspace_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(ApiError::database)?;
    let actor = workflow::current_actor(&mut tx, &session).await?;
    let assigned = if actor.role == "staff" {
        if input
            .assigned_staff_id
            .is_some_and(|id| id != actor.user_id)
        {
            return Err(ApiError::forbidden());
        }
        actor.user_id
    } else {
        sqlx::query_scalar::<_,Uuid>("SELECT id FROM matterflow.app_users WHERE workspace_id=$1 AND role='staff' AND ($2::uuid IS NULL OR id=$2) ORDER BY display_name LIMIT 1")
            .bind(actor.workspace_id).bind(input.assigned_staff_id).fetch_optional(&mut *tx).await.map_err(ApiError::database)?.ok_or_else(||ApiError::validation("Choose a staff member from this demo."))?
    };
    let fingerprint=hex::encode(Sha256::digest(serde_json::to_vec(&json!({"title":input.title,"client":input.client_label,"category":input.service_category,"assigned":assigned,"documents":input.documents})).map_err(|_|ApiError::internal())?));
    let previous:Option<(Uuid,String,String)>=sqlx::query_as("SELECT id,reference,creation_fingerprint FROM matterflow.matters WHERE workspace_id=$1 AND creation_key=$2")
        .bind(actor.workspace_id).bind(input.creation_key).fetch_optional(&mut *tx).await.map_err(ApiError::database)?;
    if let Some((id, reference, previous_fingerprint)) = previous {
        if previous_fingerprint != fingerprint {
            return Err(ApiError::conflict(
                "creation_key_reused",
                "This creation request was already used with different information. Start a new request.",
            ));
        }
        tx.commit().await.map_err(ApiError::database)?;
        return Ok((
            StatusCode::OK,
            Json(Created {
                id,
                reference,
                replayed: true,
            }),
        ));
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM matterflow.matters WHERE workspace_id=$1")
            .bind(actor.workspace_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(ApiError::database)?;
    if count >= 50 {
        return Err(ApiError::validation(
            "A demo workspace supports up to 50 matters.",
        ));
    }
    let next:i32=sqlx::query_scalar("SELECT COALESCE(max(substring(reference FROM '^MF-([0-9]+)$')::integer),0)+1 FROM matterflow.matters WHERE workspace_id=$1")
        .bind(actor.workspace_id).fetch_one(&mut *tx).await.map_err(ApiError::database)?;
    let reference = format!("MF-{next:03}");
    let reviewer:Option<Uuid>=sqlx::query_scalar("SELECT id FROM matterflow.app_users WHERE workspace_id=$1 AND role='reviewer' ORDER BY display_name LIMIT 1")
        .bind(actor.workspace_id).fetch_optional(&mut *tx).await.map_err(ApiError::database)?;
    let id:Uuid=sqlx::query_scalar("INSERT INTO matterflow.matters(workspace_id,reference,title,client_label,service_category,assigned_staff_id,assigned_reviewer_id,creation_key,creation_fingerprint,created_at,updated_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,statement_timestamp(),statement_timestamp()) RETURNING id")
        .bind(actor.workspace_id).bind(&reference).bind(&input.title).bind(&input.client_label).bind(input.service_category.as_str()).bind(assigned).bind(reviewer).bind(input.creation_key).bind(fingerprint).fetch_one(&mut *tx).await.map_err(ApiError::database)?;
    for document in &input.documents {
        sqlx::query("INSERT INTO matterflow.document_requirements(workspace_id,matter_id,title,is_required,requested_at) VALUES($1,$2,$3,$4,clock_timestamp())")
            .bind(actor.workspace_id).bind(id).bind(&document.title).bind(document.is_required).execute(&mut *tx).await.map_err(ApiError::database)?;
    }
    workflow::write_event(&mut tx,&actor,id,"matter.opened",json!({"synthetic":true,"checklist_items":input.documents.len(),"required_items":input.documents.iter().filter(|document|document.is_required).count()})).await?;
    tx.commit().await.map_err(ApiError::database)?;
    Ok((
        StatusCode::CREATED,
        Json(Created {
            id,
            reference,
            replayed: false,
        }),
    ))
}

#[derive(sqlx::FromRow, Serialize)]
pub struct StaffOption {
    pub id: Uuid,
    pub display_name: String,
}
pub async fn options(
    session: Session,
    State(state): State<AppState>,
) -> Result<Json<Vec<StaffOption>>, ApiError> {
    let staff=sqlx::query_as::<_,StaffOption>("SELECT id,display_name FROM matterflow.app_users WHERE workspace_id=$1 AND role='staff' AND ($2='reviewer' OR id=$3) ORDER BY display_name")
        .bind(session.workspace_id).bind(&session.role).bind(session.user_id).fetch_all(pool(&state)?).await.map_err(ApiError::database)?;
    Ok(Json(staff))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddRequirement {
    pub expected_version: i32,
    pub title: String,
    pub is_required: bool,
}
pub async fn add_requirement(
    session: Session,
    State(state): State<AppState>,
    Path(matter): Path<Uuid>,
    Json(input): Json<AddRequirement>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let title = input.title.split_whitespace().collect::<Vec<_>>().join(" ");
    if title.is_empty() || title.chars().count() > 200 || input.expected_version < 0 {
        return Err(ApiError::validation(
            "Provide a valid record version and a checklist title containing 1 to 200 characters.",
        ));
    }
    let mut tx = pool(&state)?.begin().await.map_err(ApiError::database)?;
    let actor = workflow::current_actor(&mut tx, &session).await?;
    let locked = workflow::lock_matter(&mut tx, &actor, matter).await?;
    if matches!(locked.status.as_str(), "ready" | "closed") {
        return Err(ApiError::conflict(
            "matter_locked",
            "Requirements cannot be added after a matter is ready or closed.",
        ));
    }
    workflow::check_version(locked.version, input.expected_version)?;
    let(count,duplicate):(i64,bool)=sqlx::query_as("SELECT count(*),COALESCE(bool_or(lower(title)=lower($3)),FALSE) FROM matterflow.document_requirements WHERE workspace_id=$1 AND matter_id=$2")
        .bind(actor.workspace_id).bind(matter).bind(&title).fetch_one(&mut *tx).await.map_err(ApiError::database)?;
    if duplicate {
        return Err(ApiError::conflict(
            "duplicate_requirement",
            "This checklist already contains a document with that title.",
        ));
    }
    if count >= 30 {
        return Err(ApiError::validation(
            "A demo matter supports up to 30 checklist items.",
        ));
    }
    let id:Uuid=sqlx::query_scalar("INSERT INTO matterflow.document_requirements(workspace_id,matter_id,title,is_required,requested_at) VALUES($1,$2,$3,$4,clock_timestamp()) RETURNING id")
        .bind(actor.workspace_id).bind(matter).bind(&title).bind(input.is_required).fetch_one(&mut *tx).await.map_err(ApiError::database)?;
    sqlx::query("UPDATE matterflow.matters SET version=version+1,updated_at=clock_timestamp(),status=CASE WHEN $3 AND status<>'intake' THEN 'collecting_documents' ELSE status END WHERE workspace_id=$1 AND id=$2")
        .bind(actor.workspace_id).bind(matter).bind(input.is_required).execute(&mut *tx).await.map_err(ApiError::database)?;
    workflow::write_event(
        &mut tx,
        &actor,
        matter,
        "requirement.added",
        json!({"requirement_id":id,"document_title":title,"is_required":input.is_required}),
    )
    .await?;
    tx.commit().await.map_err(ApiError::database)?;
    Ok((StatusCode::CREATED, Json(json!({"id":id,"version":0}))))
}
