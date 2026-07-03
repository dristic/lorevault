use std::sync::Arc;

use lv_auth::{jwt::JwtConfig, provider::AuthProvider};
use lv_storage::Storage;

use crate::config::Settings;

#[derive(Clone)]
pub struct AppState {
    pub config: Settings,
    pub storage: Arc<dyn Storage>,
    pub auth: Arc<dyn AuthProvider>,
    pub jwt: Arc<JwtConfig>,
}

impl AppState {
    pub fn new(
        config: Settings,
        storage: Arc<dyn Storage>,
        auth: Arc<dyn AuthProvider>,
        jwt: Arc<JwtConfig>,
    ) -> Self {
        Self { config, storage, auth, jwt }
    }
}
