use async_trait::async_trait;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tracing::instrument;
use uuid::Uuid;

use crate::{
    error::{AuthError, Result},
    password,
    provider::{AuthProvider, NewUser},
    validate,
};

pub struct PasswordProvider {
    db: SqlitePool,
}

impl PasswordProvider {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    #[instrument(skip(self, password_str), fields(username, email))]
    pub async fn register_with_password(
        &self,
        username: &str,
        email: &str,
        password_str: &str,
    ) -> Result<Uuid> {
        validate::username(username)?;
        validate::email(email)?;
        validate::password(password_str)?;

        let hash = password::hash(password_str)?;
        let user_id = Uuid::new_v4();
        let provider_uid = email.to_lowercase();

        let mut tx = self
            .db
            .begin()
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?;

        sqlx::query("INSERT INTO users (id, username, email) VALUES (?, ?, ?)")
            .bind(user_id)
            .bind(username)
            .bind(email)
            .execute(&mut *tx)
            .await
            .map_err(|e: sqlx::Error| match e {
                sqlx::Error::Database(ref d) if d.is_unique_violation() => {
                    let msg = d.message();
                    if msg.contains("users.username") {
                        AuthError::Conflict("username already taken".into())
                    } else {
                        AuthError::Conflict("email already registered".into())
                    }
                }
                e => AuthError::Internal(e.to_string()),
            })?;

        sqlx::query(
            "INSERT INTO user_identities (id, user_id, provider, provider_uid, credential_json)
             VALUES (?, ?, 'password', ?, ?)",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(&provider_uid)
        .bind(json!({ "hash": hash }))
        .execute(&mut *tx)
        .await
        .map_err(|e: sqlx::Error| AuthError::Internal(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?;

        Ok(user_id)
    }

    #[instrument(skip(self, password_str), fields(login))]
    pub async fn authenticate_with_password(
        &self,
        login: &str,
        password_str: &str,
    ) -> Result<Uuid> {
        let row: Option<(Uuid, Value)> = sqlx::query_as(
            r#"SELECT ui.user_id, ui.credential_json
               FROM user_identities ui
               JOIN users u ON u.id = ui.user_id
               WHERE ui.provider = 'password'
                 AND (u.username = ? OR ui.provider_uid = lower(?))"#,
        )
        .bind(login)
        .bind(login)
        .fetch_optional(&self.db)
        .await
        .map_err(|e: sqlx::Error| AuthError::Internal(e.to_string()))?;

        let (user_id, cred) = row.ok_or(AuthError::InvalidCredentials)?;

        let stored_hash = cred["hash"]
            .as_str()
            .ok_or_else(|| AuthError::Internal("malformed credential record".into()))?;

        password::verify(password_str, stored_hash)?;

        Ok(user_id)
    }
}

#[async_trait]
impl AuthProvider for PasswordProvider {
    fn name(&self) -> &'static str {
        "password"
    }

    async fn register(&self, user: NewUser, credentials: Value) -> Result<Uuid> {
        let pass = credentials["password"]
            .as_str()
            .ok_or_else(|| AuthError::Validation("password is required".into()))?;
        self.register_with_password(&user.username, &user.email, pass)
            .await
    }

    async fn authenticate(&self, credentials: Value) -> Result<Uuid> {
        let login = credentials["login"]
            .as_str()
            .ok_or_else(|| AuthError::Validation("login is required".into()))?;
        let pass = credentials["password"]
            .as_str()
            .ok_or_else(|| AuthError::Validation("password is required".into()))?;
        self.authenticate_with_password(login, pass).await
    }
}
