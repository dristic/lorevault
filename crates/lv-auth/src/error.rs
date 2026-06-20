use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("token expired")]
    TokenExpired,

    #[error("token invalid")]
    TokenInvalid,

    #[error("insufficient scope")]
    InsufficientScope,

    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, AuthError>;
