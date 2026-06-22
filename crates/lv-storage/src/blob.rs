use std::sync::Arc;

use bytes::Bytes;
use object_store::{path::Path, ObjectStore, PutPayload};

use crate::error::{Result, StorageError};

pub struct BlobStore {
    store: Arc<dyn ObjectStore>,
}

impl BlobStore {
    pub fn new(store: Arc<dyn ObjectStore>) -> Self {
        Self { store }
    }

    /// Upload a chunk. The key should be content-addressed (hash-derived).
    pub async fn put(&self, key: &str, data: Bytes) -> Result<()> {
        self.store
            .put(&Path::from(key), PutPayload::from(data))
            .await
            .map_err(|e| StorageError::Blob(e.to_string()))?;
        Ok(())
    }

    pub async fn get(&self, key: &str) -> Result<Bytes> {
        self.store
            .get(&Path::from(key))
            .await
            .map_err(|e| StorageError::Blob(e.to_string()))?
            .bytes()
            .await
            .map_err(|e| StorageError::Blob(e.to_string()))
    }

    pub async fn exists(&self, key: &str) -> Result<bool> {
        match self.store.head(&Path::from(key)).await {
            Ok(_) => Ok(true),
            Err(object_store::Error::NotFound { .. }) => Ok(false),
            Err(e) => Err(StorageError::Blob(e.to_string())),
        }
    }
}
