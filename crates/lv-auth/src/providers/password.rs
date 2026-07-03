use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::instrument;
use uuid::Uuid;

use lv_storage::Storage;

use crate::{
    error::{AuthError, Result},
    password,
    provider::{AuthProvider, NewUser},
    validate,
};

pub struct PasswordProvider {
    storage: Arc<dyn Storage>,
}

impl PasswordProvider {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
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
        let credential = serde_json::to_string(&json!({ "hash": hash }))
            .map_err(|e| AuthError::Internal(e.to_string()))?;

        let mut tx = self.storage.begin().await?;
        tx.insert_user(user_id, username, email).await?;
        tx.insert_user_identity(Uuid::new_v4(), user_id, "password", &provider_uid, &credential)
            .await?;
        tx.commit().await?;

        Ok(user_id)
    }

    #[instrument(skip(self, password_str), fields(login))]
    pub async fn authenticate_with_password(
        &self,
        login: &str,
        password_str: &str,
    ) -> Result<Uuid> {
        let (user, cred_str) = self
            .storage
            .get_user_identity_by_login("password", login)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        let cred: Value = serde_json::from_str(&cred_str)
            .map_err(|_| AuthError::Internal("malformed credential record".into()))?;

        let stored_hash = cred["hash"]
            .as_str()
            .ok_or_else(|| AuthError::Internal("malformed credential record".into()))?;

        password::verify(password_str, stored_hash)?;

        Ok(user.id)
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
