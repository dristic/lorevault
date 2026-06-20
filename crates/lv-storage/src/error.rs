use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("blob storage error: {0}")]
    Blob(String),

    #[error("cache error: {0}")]
    Cache(String),

    #[error("not found")]
    NotFound,
}

pub type Result<T> = std::result::Result<T, StorageError>;
