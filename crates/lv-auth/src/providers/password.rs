use async_trait::async_trait;
use serde_json::{json, Value};
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::{
    error::{AuthError, Result},
    password,
    provider::{AuthProvider, NewUser},
    validate,
};

/// Username-and-password authentication.
///
/// Credentials are stored in `user_identities` with `provider = 'password'`
/// and `provider_uid = lower(email)`. The `credential_json` column holds
/// `{ "hash": "<argon2 PHC string>" }`.
///
/// # `register` credentials shape
/// ```json
/// { "password": "correct-horse-battery-staple" }
/// ```
///
/// # `authenticate` credentials shape
/// ```json
/// { "login": "alice",  "password": "..." }
/// ```
/// `login` is matched against both `users.username` and the identity's
/// `provider_uid` (normalised email), so users can sign in with either.
pub struct PasswordProvider {
    db: PgPool,
}

impl PasswordProvider {
    pub fn new(db: PgPool) -> Self {
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

        sqlx::query("INSERT INTO users (id, username, email) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(username)
            .bind(email)
            .execute(&mut *tx)
            .await
            .map_err(|e: sqlx::Error| match e {
                sqlx::Error::Database(ref d)
                    if d.constraint() == Some("users_username_key") =>
                {
                    AuthError::Conflict("username already taken".into())
                }
                sqlx::Error::Database(ref d)
                    if d.constraint() == Some("users_email_key") =>
                {
                    AuthError::Conflict("email already registered".into())
                }
                e => AuthError::Internal(e.to_string()),
            })?;

        sqlx::query(
            "INSERT INTO user_identities (id, user_id, provider, provider_uid, credential_json)
             VALUES ($1, $2, 'password', $3, $4)",
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

    /// Accepts username or email as `login`.
    #[instrument(skip(self, password_str), fields(login))]
    pub async fn authenticate_with_password(
        &self,
        login: &str,
        password_str: &str,
    ) -> Result<Uuid> {
        // Match on username OR normalised email (provider_uid).
        let row: Option<(Uuid, Value)> = sqlx::query_as(
            r#"SELECT ui.user_id, ui.credential_json
               FROM user_identities ui
               JOIN users u ON u.id = ui.user_id
               WHERE ui.provider = 'password'
                 AND (u.username = $1 OR ui.provider_uid = lower($1))"#,
        )
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
