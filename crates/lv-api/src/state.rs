use std::sync::Arc;

use sqlx::AnyPool;

use lv_auth::{jwt::JwtConfig, provider::AuthProvider};

use crate::config::Settings;

#[derive(Clone)]
pub struct AppState {
    pub config: Settings,
    pub db: AnyPool,
    pub auth: Arc<dyn AuthProvider>,
    pub jwt: Arc<JwtConfig>,
}

impl AppState {
    pub fn new(config: Settings, db: AnyPool, auth: Arc<dyn AuthProvider>, jwt: Arc<JwtConfig>) -> Self {
        Self { config, db, auth, jwt }
    }
}
