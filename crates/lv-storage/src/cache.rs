use deadpool_redis::{Config, Pool, Runtime};

use crate::error::{Result, StorageError};

pub fn connect(url: &str) -> Result<Pool> {
    let cfg = Config::from_url(url);
    cfg.create_pool(Some(Runtime::Tokio1))
        .map_err(|e| StorageError::Cache(e.to_string()))
}
