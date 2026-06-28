use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("not found")]
    NotFound,
}

pub type Result<T> = std::result::Result<T, StorageError>;
