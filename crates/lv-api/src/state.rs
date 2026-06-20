use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;

use crate::config::Settings;

#[derive(Clone)]
pub struct AppState {
    pub config: Settings,
    pub db: PgPool,
    pub cache: RedisPool,
}

impl AppState {
    pub fn new(config: Settings, db: PgPool, cache: RedisPool) -> Self {
        Self { config, db, cache }
    }
}
