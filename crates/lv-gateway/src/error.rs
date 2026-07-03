use lv_storage::StorageError;
use tonic::Status;

pub fn map_storage_err(e: StorageError) -> Status {
    match e {
        StorageError::NotFound => Status::not_found("not found"),
        StorageError::UniqueViolation { field } => {
            Status::already_exists(format!("conflict: {field}"))
        }
        StorageError::ForeignKeyViolation => Status::failed_precondition("foreign key violation"),
        other => Status::internal(other.to_string()),
    }
}
