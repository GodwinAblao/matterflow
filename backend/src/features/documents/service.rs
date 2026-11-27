use super::{ReviewInput, ReviewOutcome};
use crate::{
    error::ApiError,
    features::{
        auth::Session,
        matters::workflow::{self, ChangeResult},
    },
};
use serde_json::json;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(FromRow)]
struct Document {
    title: String,
    status: String,
    version: i32,
}

enum Action {
    Receive,
    Review {
        outcome: ReviewOutcome,
        note: Option<String>,
    },
}

pub async fn receive(
    pool: &PgPool,
    session: &Session,
    matter: Uuid,
    id: Uuid,
    version: i32,
) -> Result<ChangeResult, ApiError> {
    change(pool, session, matter, id, version, Action::Receive).await
}

pub async fn review(
    pool: &PgPool,
    session: &Session,
    matter: Uuid,
    id: Uuid,
    input: ReviewInput,
) -> Result<ChangeResult, ApiError> {
    let note = input
        .note
        .map(|note| note.trim().to_owned())
        .filter(|note| !note.is_empty());
    match input.outcome {
        ReviewOutcome::NeedsCorrection
            if note.as_ref().is_none_or(|note| note.chars().count() > 1000) =>
        {
            return Err(ApiError::validation(
                "A correction explanation between 1 and 1,000 characters is required.",
            ));
        }
        ReviewOutcome::Accepted if note.is_some() => {
            return Err(ApiError::validation(
                "An acceptance cannot include a correction explanation.",
            ));
        }
        _ => {}
    }
    change(
        pool,
        session,
        matter,
        id,
        input.expected_version,
        Action::Review {
            outcome: input.outcome,
            note,
        },
    )
    .await
}

async fn change(
    pool: &PgPool,
    session: &Session,
    matter_id: Uuid,
    id: Uuid,
    expected: i32,
    action: Action,
) -> Result<ChangeResult, ApiError> {
    if expected < 0 {
        return Err(ApiError::validation("A valid record version is required."));
    }
    let mut tx = pool.begin().await.map_err(ApiError::database)?;
    let actor = workflow::current_actor(&mut tx, session).await?;
    match &action {
        Action::Receive if actor.role != "staff" => return Err(ApiError::forbidden()),
        Action::Review { .. } => actor.require_reviewer()?,
        _ => {}
    }
    let matter = workflow::lock_matter(&mut tx, &actor, matter_id).await?;
    if matches!(matter.status.as_str(), "ready" | "closed") {
        return Err(ApiError::conflict(
            "matter_locked",
            "Documents cannot be changed after a matter is ready or closed.",
        ));
    }
    let document=sqlx::query_as::<_,Document>("SELECT title,status,version FROM matterflow.document_requirements WHERE workspace_id=$1 AND matter_id=$2 AND id=$3 FOR UPDATE")
        .bind(actor.workspace_id).bind(matter_id).bind(id).fetch_optional(&mut *tx).await.map_err(ApiError::database)?.ok_or_else(ApiError::not_found)?;
    workflow::check_version(document.version, expected)?;
    let (target, note, event) = match action {
        Action::Receive => {
            if !matches!(document.status.as_str(), "missing" | "needs_correction") {
                return Err(ApiError::conflict(
                    "invalid_transition",
                    "Only missing or correction-needed documents can be recorded as received.",
                ));
            }
            (
                "received",
                None,
                if document.status == "needs_correction" {
                    "document.resubmitted"
                } else {
                    "document.received"
                },
            )
        }
        Action::Review { outcome, note } => {
            if document.status != "received" {
                return Err(ApiError::conflict(
                    "invalid_transition",
                    "Only received documents can be reviewed.",
                ));
            }
            match outcome {
                ReviewOutcome::Accepted => ("accepted", None, "document.accepted"),
                ReviewOutcome::NeedsCorrection => {
                    ("needs_correction", note, "document.correction_requested")
                }
            }
        }
    };
    sqlx::query("UPDATE matterflow.document_requirements SET status=$4, received_at=CASE WHEN $4='received' THEN clock_timestamp() ELSE received_at END, reviewed_at=CASE WHEN $4='received' THEN NULL ELSE clock_timestamp() END, reviewed_by=CASE WHEN $4='received' THEN NULL ELSE $5 END,correction_note=$6,version=version+1 WHERE workspace_id=$1 AND matter_id=$2 AND id=$3")
        .bind(actor.workspace_id).bind(matter_id).bind(id).bind(target).bind(actor.user_id).bind(&note)
        .execute(&mut *tx).await.map_err(ApiError::database)?;
    // Every checklist mutation shares the matter lock with readiness confirmation.
    let (status,version):(String,i32)=sqlx::query_as("UPDATE matterflow.matters SET status=CASE WHEN EXISTS(SELECT 1 FROM matterflow.document_requirements WHERE workspace_id=$1 AND matter_id=$2 AND is_required AND status IN ('missing','needs_correction')) THEN 'collecting_documents' ELSE 'under_review' END,updated_at=clock_timestamp(),version=version+1 WHERE workspace_id=$1 AND id=$2 RETURNING status,version")
        .bind(actor.workspace_id).bind(matter_id).fetch_one(&mut *tx).await.map_err(ApiError::database)?;
    workflow::write_event(&mut tx,&actor,matter_id,event,json!({"requirement_id":id,"document_title":document.title,"from_status":document.status,"to_status":target,"note":note,"document_version":expected+1,"matter_stage_from":matter.status,"matter_stage_to":status})).await?;
    tx.commit().await.map_err(ApiError::database)?;
    Ok(ChangeResult {
        matter_id,
        status,
        version,
    })
}
