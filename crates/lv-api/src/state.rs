use std::sync::Arc;

use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;

use lv_auth::provider::AuthProvider;

use crate::config::Settings;

#[derive(Clone)]
pub struct AppState {
    pub config: Settings,
    pub db: PgPool,
    pub cache: RedisPool,
    /// Active authentication provider. Swap for a different `AuthProvider`
    /// implementation to support OAuth, SAML, etc. without touching route logic.
    pub auth: Arc<dyn AuthProvider>,
}

impl AppState {
    pub fn new(
        config: Settings,
        db: PgPool,
        cache: RedisPool,
        auth: Arc<dyn AuthProvider>,
    ) -> Self {
        Self {
            config,
            db,
            cache,
            auth,
        }
    }
}
