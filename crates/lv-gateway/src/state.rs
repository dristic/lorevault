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
    /// Hostname extracted from `server_url`, used as JWT `iss` and `aud`.
    pub issuer: String,
}

impl GatewayState {
    pub fn new(
        db: PgPool,
        blob: Arc<BlobStore>,
        jwt_secret: String,
        jwt_ttl_secs: i64,
        server_url: String,
    ) -> Self {
        let issuer = server_url
            .split_once("://")
            .map(|(_, rest)| rest.split(':').next().unwrap_or(rest).to_string())
            .unwrap_or_else(|| server_url.clone());
        Self {
            db,
            blob,
            jwt_secret,
            jwt_ttl_secs,
            server_url,
            issuer,
        }
    }
}
