use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};

use crate::error::Result;

pub async fn connect(url: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);

    SqlitePoolOptions::new()
        .max_connections(16)
        .connect_with(options)
        .await
        .map_err(Into::into)
}
