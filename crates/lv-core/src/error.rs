use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("not found: {resource} {id}")]
    NotFound { resource: &'static str, id: String },

    #[error("permission denied")]
    PermissionDenied,

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
