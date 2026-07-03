use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("not found")]
    NotFound,

    #[error("unique constraint violation on {field}")]
    UniqueViolation { field: &'static str },

    #[error("foreign key violation")]
    ForeignKeyViolation,

    #[error("backend error")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub type Result<T> = std::result::Result<T, StorageError>;
