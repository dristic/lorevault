use sqlx::AnyPool;

use crate::error::Result;

pub async fn connect(url: &str) -> Result<AnyPool> {
    // Register all compiled-in database drivers (sqlite + any others present in the dep graph).
    sqlx::any::install_default_drivers();

    let pool = AnyPool::connect(url).await?;

    // SQLite-specific tuning: WAL journal mode + enforce foreign-key constraints.
    if url.starts_with("sqlite:") {
        sqlx::query("PRAGMA journal_mode=WAL").execute(&pool).await?;
        sqlx::query("PRAGMA foreign_keys=ON").execute(&pool).await?;
    }

    Ok(pool)
}
