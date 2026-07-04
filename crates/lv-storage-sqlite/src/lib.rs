use std::str::FromStr;

use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use sqlx::SqliteConnection;
use time::OffsetDateTime;
use uuid::fmt::Hyphenated;
use uuid::Uuid;

use lv_core::models::{AuthSession, Repository, RepoRole, User, Visibility};
use lv_storage::{Result, Storage, StorageError, StorageTx};

/// Opens (creating if missing) the SQLite database at `url` and configures it
/// for concurrent access (WAL) and referential integrity.
pub async fn connect(url: &str) -> sqlx::Result<SqlitePool> {
    let opts = SqliteConnectOptions::from_str(url)?.create_if_missing(true);
    if let Some(parent) = opts.get_filename().parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(sqlx::Error::Io)?;
        }
    }

    let pool = SqlitePool::connect_with(opts).await?;
    sqlx::query("PRAGMA journal_mode=WAL").execute(&pool).await?;
    sqlx::query("PRAGMA foreign_keys=ON").execute(&pool).await?;

    Ok(pool)
}

#[derive(Debug)]
struct TxError(&'static str);

impl std::fmt::Display for TxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for TxError {}

/// Maps a raw `sqlx::Error` to a `StorageError`, centralizing the one place
/// that inspects backend-specific constraint-violation messages.
fn map_sqlx_err(e: sqlx::Error) -> StorageError {
    if let sqlx::Error::Database(ref db_err) = e {
        if db_err.is_unique_violation() {
            let msg = db_err.message();
            let field = if msg.contains("users.username") {
                "username"
            } else if msg.contains("users.email") {
                "email"
            } else if msg.contains("user_identities") {
                "provider_uid"
            } else if msg.contains("repositories") {
                "name"
            } else if msg.contains("api_tokens") {
                "token_hash"
            } else {
                "unknown"
            };
            return StorageError::UniqueViolation { field };
        }
        if db_err.is_foreign_key_violation() {
            return StorageError::ForeignKeyViolation;
        }
    }
    StorageError::Backend(Box::new(e))
}

fn row_to_user(
    id: Hyphenated,
    username: String,
    email: String,
    is_admin: bool,
    must_change_password: bool,
    created_at: OffsetDateTime,
) -> User {
    User {
        id: id.into_uuid(),
        username,
        email,
        is_admin,
        must_change_password,
        created_at,
    }
}

pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Storage for SqliteStorage {
    async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let row: Option<(Hyphenated, String, String, bool, bool, OffsetDateTime)> = sqlx::query_as(
            "SELECT id, username, email, is_admin, must_change_password, created_at FROM users WHERE id = ?",
        )
        .bind(id.hyphenated())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.map(|(id, username, email, is_admin, must_change_password, created_at)| {
            row_to_user(id, username, email, is_admin, must_change_password, created_at)
        }))
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let row: Option<(Hyphenated, String, String, bool, bool, OffsetDateTime)> = sqlx::query_as(
            "SELECT id, username, email, is_admin, must_change_password, created_at FROM users WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.map(|(id, username, email, is_admin, must_change_password, created_at)| {
            row_to_user(id, username, email, is_admin, must_change_password, created_at)
        }))
    }

    async fn get_user_identity_by_login(
        &self,
        provider: &str,
        login: &str,
    ) -> Result<Option<(User, String)>> {
        let row: Option<(Hyphenated, String, String, bool, bool, OffsetDateTime, String)> = sqlx::query_as(
            r#"SELECT u.id, u.username, u.email, u.is_admin, u.must_change_password, u.created_at, ui.credential_json
               FROM user_identities ui
               JOIN users u ON u.id = ui.user_id
               WHERE ui.provider = ?
                 AND (u.username = ? OR ui.provider_uid = lower(?))"#,
        )
        .bind(provider)
        .bind(login)
        .bind(login)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.map(|(id, username, email, is_admin, must_change_password, created_at, credential_json)| {
            (
                row_to_user(id, username, email, is_admin, must_change_password, created_at),
                credential_json,
            )
        }))
    }

    async fn insert_api_token(
        &self,
        id: Uuid,
        user_id: Uuid,
        name: &str,
        token_hash: &str,
    ) -> Result<()> {
        sqlx::query("INSERT INTO api_tokens (id, user_id, name, token_hash) VALUES (?, ?, ?, ?)")
            .bind(id.hyphenated())
            .bind(user_id.hyphenated())
            .bind(name)
            .bind(token_hash)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        Ok(())
    }

    async fn find_user_by_token_hash(&self, token_hash: &str) -> Result<Option<User>> {
        let row: Option<(Hyphenated, String, String, bool, bool, OffsetDateTime)> = sqlx::query_as(
            r#"SELECT u.id, u.username, u.email, u.is_admin, u.must_change_password, u.created_at
               FROM api_tokens t
               JOIN users u ON u.id = t.user_id
               WHERE t.token_hash = ?"#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.map(|(id, username, email, is_admin, must_change_password, created_at)| {
            row_to_user(id, username, email, is_admin, must_change_password, created_at)
        }))
    }

    async fn get_repository_by_owner_and_name(
        &self,
        owner_username: &str,
        repo_name: &str,
    ) -> Result<Option<Repository>> {
        let row: Option<(Hyphenated, Hyphenated, String, Option<String>, Visibility, String, OffsetDateTime)> =
            sqlx::query_as(
                r#"SELECT r.id, r.owner_id, r.name, r.description, r.visibility, r.default_branch, r.created_at
                   FROM repositories r
                   JOIN users u ON r.owner_id = u.id
                   WHERE u.username = ? AND r.name = ?"#,
            )
            .bind(owner_username)
            .bind(repo_name)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;

        Ok(row.map(
            |(id, owner_id, name, description, visibility, default_branch, created_at)| Repository {
                id: id.into_uuid(),
                owner_id: owner_id.into_uuid(),
                name,
                description,
                visibility,
                default_branch,
                created_at,
            },
        ))
    }

    async fn get_repo_permission(&self, repo_id: Uuid, user_id: Uuid) -> Result<Option<RepoRole>> {
        let role: Option<RepoRole> =
            sqlx::query_scalar("SELECT role FROM repo_permissions WHERE repo_id = ? AND user_id = ?")
                .bind(repo_id.hyphenated())
                .bind(user_id.hyphenated())
                .fetch_optional(&self.pool)
                .await
                .map_err(map_sqlx_err)?;
        Ok(role)
    }

    async fn delete_repository(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM repositories WHERE id = ?")
            .bind(id.hyphenated())
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        Ok(())
    }

    async fn start_auth_session(&self, code: &str, expires_at: OffsetDateTime) -> Result<()> {
        sqlx::query("INSERT INTO auth_sessions (code, expires_at) VALUES (?, ?)")
            .bind(code)
            .bind(expires_at.unix_timestamp())
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        Ok(())
    }

    async fn get_auth_session(&self, code: &str, now: OffsetDateTime) -> Result<Option<AuthSession>> {
        use lv_core::models::AuthSessionState;

        let row: Option<(String, AuthSessionState, Option<String>, Option<Hyphenated>, Option<String>, i64)> =
            sqlx::query_as(
                "SELECT code, state, token, user_id, username, expires_at \
                 FROM auth_sessions WHERE code = ? AND expires_at > ?",
            )
            .bind(code)
            .bind(now.unix_timestamp())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;

        row.map(|(code, state, token, user_id, username, expires_at)| {
            Ok(AuthSession {
                code,
                state,
                token,
                user_id: user_id.map(Hyphenated::into_uuid),
                username,
                expires_at: OffsetDateTime::from_unix_timestamp(expires_at)
                    .map_err(|e| StorageError::Backend(Box::new(e)))?,
            })
        })
        .transpose()
    }

    async fn complete_auth_session(
        &self,
        code: &str,
        token: &str,
        user_id: Uuid,
        username: &str,
        expires_at: OffsetDateTime,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE auth_sessions SET state = 'complete', token = ?, user_id = ?, username = ?, expires_at = ? \
             WHERE code = ?",
        )
        .bind(token)
        .bind(user_id.hyphenated())
        .bind(username)
        .bind(expires_at.unix_timestamp())
        .bind(code)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }

    async fn ping(&self) -> Result<()> {
        // The pool is a local file handle — if it's open, it's healthy.
        // No round-trip needed, unlike a networked backend (e.g. Postgres).
        Ok(())
    }

    async fn begin(&self) -> Result<Box<dyn StorageTx>> {
        let tx = self.pool.begin().await.map_err(map_sqlx_err)?;
        Ok(Box::new(SqliteStorageTx { tx: Some(tx) }))
    }
}

struct SqliteStorageTx {
    tx: Option<sqlx::Transaction<'static, sqlx::Sqlite>>,
}

impl SqliteStorageTx {
    fn conn(&mut self) -> Result<&mut SqliteConnection> {
        self.tx
            .as_deref_mut()
            .ok_or_else(|| StorageError::Backend(Box::new(TxError("transaction already committed"))))
    }
}

#[async_trait]
impl StorageTx for SqliteStorageTx {
    async fn insert_user(&mut self, id: Uuid, username: &str, email: &str) -> Result<()> {
        let conn = self.conn()?;
        sqlx::query("INSERT INTO users (id, username, email) VALUES (?, ?, ?)")
            .bind(id.hyphenated())
            .bind(username)
            .bind(email)
            .execute(conn)
            .await
            .map_err(map_sqlx_err)?;
        Ok(())
    }

    async fn insert_user_identity(
        &mut self,
        id: Uuid,
        user_id: Uuid,
        provider: &str,
        provider_uid: &str,
        credential_json: &str,
    ) -> Result<()> {
        let conn = self.conn()?;
        sqlx::query(
            "INSERT INTO user_identities (id, user_id, provider, provider_uid, credential_json) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id.hyphenated())
        .bind(user_id.hyphenated())
        .bind(provider)
        .bind(provider_uid)
        .bind(credential_json)
        .execute(conn)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }

    async fn insert_repository(
        &mut self,
        id: Uuid,
        owner_id: Uuid,
        name: &str,
        visibility: Visibility,
    ) -> Result<()> {
        let conn = self.conn()?;
        sqlx::query(
            "INSERT INTO repositories (id, owner_id, name, visibility, default_branch) \
             VALUES (?, ?, ?, ?, 'main')",
        )
        .bind(id.hyphenated())
        .bind(owner_id.hyphenated())
        .bind(name)
        .bind(visibility)
        .execute(conn)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }

    async fn insert_repo_permission(&mut self, repo_id: Uuid, user_id: Uuid, role: RepoRole) -> Result<()> {
        let conn = self.conn()?;
        sqlx::query("INSERT INTO repo_permissions (repo_id, user_id, role) VALUES (?, ?, ?)")
            .bind(repo_id.hyphenated())
            .bind(user_id.hyphenated())
            .bind(role)
            .execute(conn)
            .await
            .map_err(map_sqlx_err)?;
        Ok(())
    }

    async fn update_user_credential(
        &mut self,
        user_id: Uuid,
        provider: &str,
        credential_json: &str,
    ) -> Result<()> {
        let conn = self.conn()?;
        sqlx::query("UPDATE user_identities SET credential_json = ? WHERE user_id = ? AND provider = ?")
            .bind(credential_json)
            .bind(user_id.hyphenated())
            .bind(provider)
            .execute(conn)
            .await
            .map_err(map_sqlx_err)?;
        Ok(())
    }

    async fn set_admin_flags(
        &mut self,
        user_id: Uuid,
        is_admin: bool,
        must_change_password: bool,
    ) -> Result<()> {
        let conn = self.conn()?;
        sqlx::query("UPDATE users SET is_admin = ?, must_change_password = ? WHERE id = ?")
            .bind(is_admin)
            .bind(must_change_password)
            .bind(user_id.hyphenated())
            .execute(conn)
            .await
            .map_err(map_sqlx_err)?;
        Ok(())
    }

    async fn commit(&mut self) -> Result<()> {
        let tx = self
            .tx
            .take()
            .ok_or_else(|| StorageError::Backend(Box::new(TxError("transaction already committed"))))?;
        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }
}
