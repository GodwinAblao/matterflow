use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: &'static str,
    pub details: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorBody {
                error: ErrorDetail {
                    code: self.code,
                    message: self.message,
                    details: self.details,
                },
            }),
        )
            .into_response()
    }
}

impl ApiError {
    pub fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "session_expired",
            message: "Your demo session is missing or expired. Start a new demo to continue.",
            details: None,
        }
    }
    pub fn forbidden() -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: "forbidden",
            message: "Your current role cannot perform this action.",
            details: None,
        }
    }
    pub fn unavailable() -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "demo_unavailable",
            message: "The demo is temporarily unavailable. Please try again shortly.",
            details: None,
        }
    }
    pub fn capacity() -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: "demo_capacity",
            message: "All demo spaces are in use. Please try again later.",
            details: None,
        }
    }
    pub fn internal() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: "The request could not be completed.",
            details: None,
        }
    }
    pub fn not_found() -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message: "The requested record was not found.",
            details: None,
        }
    }
    pub fn database(error: sqlx::Error) -> Self {
        tracing::error!(code = ?error.as_database_error().and_then(|error| error.code()), "Database operation failed");
        Self::unavailable()
    }
}

impl ApiError {
    pub fn conflict(code: &'static str, message: &'static str) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            code,
            message,
            details: None,
        }
    }
    pub fn validation(message: &'static str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_input",
            message,
            details: None,
        }
    }
}
