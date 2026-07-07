use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use thiserror::Error;

use lv_auth::error::AuthError;
use lv_storage::StorageError;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("not found")]
    NotFound,

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("internal server error")]
    Internal(#[from] anyhow::Error),
}

impl From<AuthError> for ApiError {
    fn from(e: AuthError) -> Self {
        match e {
            AuthError::InvalidCredentials | AuthError::TokenExpired | AuthError::TokenInvalid => {
                ApiError::Unauthorized
            }
            AuthError::InsufficientScope => ApiError::Forbidden,
            AuthError::Validation(msg) => ApiError::BadRequest(msg),
            AuthError::Conflict(msg) => ApiError::Conflict(msg),
            AuthError::NotSupported => {
                ApiError::BadRequest("operation not supported by this auth provider".into())
            }
            AuthError::Internal(msg) => ApiError::Internal(anyhow::anyhow!(msg)),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, code) = match &self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            ApiError::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            ApiError::Internal(e) => {
                tracing::error!("internal server error: {e:#}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal")
            }
        };
        (
            status,
            Json(json!({ "error": self.to_string(), "code": code })),
        )
            .into_response()
    }
}

pub type Result<T> = std::result::Result<T, ApiError>;

impl From<StorageError> for ApiError {
    fn from(e: StorageError) -> Self {
        match e {
            StorageError::NotFound => ApiError::NotFound,
            StorageError::UniqueViolation { field } => ApiError::Conflict(field.to_string()),
            StorageError::Validation(msg) => ApiError::BadRequest(msg),
            other => ApiError::Internal(other.into()),
        }
    }
}
