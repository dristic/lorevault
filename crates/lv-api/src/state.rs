use std::sync::Arc;

use sqlx::AnyPool;

use lv_auth::provider::AuthProvider;

use crate::config::Settings;

#[derive(Clone)]
pub struct AppState {
    pub config: Settings,
    pub db: AnyPool,
    pub auth: Arc<dyn AuthProvider>,
}

impl AppState {
    pub fn new(config: Settings, db: AnyPool, auth: Arc<dyn AuthProvider>) -> Self {
        Self { config, db, auth }
    }
}
