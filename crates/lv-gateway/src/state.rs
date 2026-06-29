use std::sync::Arc;

use lv_auth::jwt::JwtConfig;
use sqlx::AnyPool;

#[derive(Clone)]
pub struct GatewayState {
    pub db: AnyPool,
    pub jwt: Arc<JwtConfig>,
    pub server_url: String,
    pub web_url: String,
}

impl GatewayState {
    pub fn new(
        db: AnyPool,
        jwt: Arc<JwtConfig>,
        server_url: String,
        web_url: String,
    ) -> Self {
        Self { db, jwt, server_url, web_url }
    }
}
