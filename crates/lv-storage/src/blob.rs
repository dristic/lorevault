use aws_sdk_s3::{primitives::ByteStream, Client};
use bytes::Bytes;

use crate::error::{Result, StorageError};

pub struct BlobStore {
    client: Client,
    bucket: String,
}

impl BlobStore {
    pub fn new(client: Client, bucket: String) -> Self {
        Self { client, bucket }
    }

    /// Upload a chunk. The key should be content-addressed (hash-derived).
    pub async fn put(&self, key: &str, data: Bytes) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(data))
            .send()
            .await
            .map_err(|e| StorageError::Blob(e.to_string()))?;
        Ok(())
    }

    pub async fn get(&self, key: &str) -> Result<Bytes> {
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError::Blob(e.to_string()))?;

        let data = resp
            .body
            .collect()
            .await
            .map_err(|e| StorageError::Blob(e.to_string()))?
            .into_bytes();
        Ok(data)
    }

    pub async fn exists(&self, key: &str) -> Result<bool> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.as_service_error()
                    .map(|s| s.is_not_found())
                    .unwrap_or(false)
                {
                    Ok(false)
                } else {
                    Err(StorageError::Blob(e.to_string()))
                }
            }
        }
    }
}
