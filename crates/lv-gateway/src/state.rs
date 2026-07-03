use std::sync::Arc;

use lv_auth::jwt::JwtConfig;
use lv_storage::Storage;

#[derive(Clone)]
pub struct GatewayState {
    pub storage: Arc<dyn Storage>,
    pub jwt: Arc<JwtConfig>,
    pub server_url: String,
    pub web_url: String,
}

impl GatewayState {
    pub fn new(
        storage: Arc<dyn Storage>,
        jwt: Arc<JwtConfig>,
        server_url: String,
        web_url: String,
    ) -> Self {
        Self { storage, jwt, server_url, web_url }
    }
}
