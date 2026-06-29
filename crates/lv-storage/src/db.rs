use sqlx::AnyPool;

use crate::error::Result;

pub async fn connect(url: &str) -> Result<AnyPool> {
    sqlx::any::install_default_drivers();

    if url.starts_with("sqlite:") {
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
        use std::str::FromStr;

        let sqlite_opts = SqliteConnectOptions::from_str(url)?.create_if_missing(true);
        if let Some(parent) = sqlite_opts.get_filename().parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        // Briefly open via SqlitePool to create the file, then hand off to AnyPool.
        SqlitePool::connect_with(sqlite_opts).await?.close().await;
    }

    let pool = AnyPool::connect(url).await?;

    if url.starts_with("sqlite:") {
        sqlx::query("PRAGMA journal_mode=WAL").execute(&pool).await?;
        sqlx::query("PRAGMA foreign_keys=ON").execute(&pool).await?;
    }

    Ok(pool)
}
