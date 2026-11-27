pub mod service;

use crate::{
    error::ApiError,
    features::{
        auth::{Session, session::pool},
        matters::workflow::ChangeResult,
    },
    state::AppState,
};
use axum::{
    Json,
    extract::{Path, State},
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VersionInput {
    pub expected_version: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewOutcome {
    Accepted,
    NeedsCorrection,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewInput {
    pub expected_version: i32,
    pub outcome: ReviewOutcome,
    pub note: Option<String>,
}

pub async fn receive(
    session: Session,
    State(state): State<AppState>,
    Path((matter_id, id)): Path<(Uuid, Uuid)>,
    Json(input): Json<VersionInput>,
) -> Result<Json<ChangeResult>, ApiError> {
    Ok(Json(
        service::receive(
            pool(&state)?,
            &session,
            matter_id,
            id,
            input.expected_version,
        )
        .await?,
    ))
}

pub async fn review(
    session: Session,
    State(state): State<AppState>,
    Path((matter_id, id)): Path<(Uuid, Uuid)>,
    Json(input): Json<ReviewInput>,
) -> Result<Json<ChangeResult>, ApiError> {
    Ok(Json(
        service::review(pool(&state)?, &session, matter_id, id, input).await?,
    ))
}
