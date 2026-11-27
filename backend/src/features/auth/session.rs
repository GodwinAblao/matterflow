use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use chrono::{DateTime, Utc};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Staff,
    Reviewer,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Staff => "staff",
            Self::Reviewer => "reviewer",
        }
    }
}

#[derive(Clone, FromRow, Serialize)]
pub struct Session {
    #[serde(skip)]
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub user_id: Uuid,
    pub display_name: String,
    pub role: String,
    pub expires_at: DateTime<Utc>,
}

impl Session {
    pub fn require_reviewer(&self) -> Result<(), ApiError> {
        if self.role == "reviewer" {
            Ok(())
        } else {
            Err(ApiError::forbidden())
        }
    }
}

pub fn token_hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

pub fn new_token() -> Result<String, ApiError> {
    let mut bytes = [0_u8; 32];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|_| ApiError::internal())?;
    Ok(hex::encode(bytes))
}

pub fn pool(state: &AppState) -> Result<&PgPool, ApiError> {
    state.pool.as_ref().ok_or_else(ApiError::unavailable)
}

impl FromRequestParts<AppState> for Session {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .ok_or_else(ApiError::unauthorized)?;
        sqlx::query_as::<_, Session>(
            "SELECT s.id, s.workspace_id, s.user_id, u.display_name, u.role, s.expires_at
             FROM matterflow.demo_sessions s
             JOIN matterflow.app_users u ON (u.workspace_id, u.id) = (s.workspace_id, s.user_id)
             JOIN matterflow.workspaces w ON w.id = s.workspace_id
             WHERE s.token_hash = $1 AND s.expires_at > now()
               AND w.expires_at > now() AND NOT w.is_template",
        )
        .bind(token_hash(token))
        .fetch_optional(pool(state)?)
        .await
        .map_err(ApiError::database)?
        .ok_or_else(ApiError::unauthorized)
    }
}
