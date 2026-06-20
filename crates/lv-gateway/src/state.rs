use std::sync::Arc;

use sqlx::PgPool;

use lv_storage::blob::BlobStore;

#[derive(Clone)]
pub struct GatewayState {
    pub db: PgPool,
    pub blob: Arc<BlobStore>,
    pub jwt_secret: String,
    pub jwt_ttl_secs: i64,
    pub server_url: String,
}

impl GatewayState {
    pub fn new(
        db: PgPool,
        blob: Arc<BlobStore>,
        jwt_secret: String,
        jwt_ttl_secs: i64,
        server_url: String,
    ) -> Self {
        Self {
            db,
            blob,
            jwt_secret,
            jwt_ttl_secs,
            server_url,
        }
    }
}
