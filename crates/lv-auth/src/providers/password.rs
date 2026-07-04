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
        tx.insert_user_identity(
            Uuid::new_v4(),
            user_id,
            "password",
            &provider_uid,
            &credential,
        )
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

    /// Verifies `current_password`, then replaces the stored hash and clears
    /// `must_change_password`. `is_admin` is passed through unchanged since
    /// `set_admin_flags` sets both flags to absolute values, not a delta.
    #[instrument(skip(self, current_password, new_password), fields(%user_id))]
    async fn change_password_impl(
        &self,
        user_id: Uuid,
        current_password: &str,
        new_password: &str,
    ) -> Result<()> {
        validate::password(new_password)?;

        let user = self
            .storage
            .get_user_by_id(user_id)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        let (_, cred_str) = self
            .storage
            .get_user_identity_by_login("password", &user.username)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        let cred: Value = serde_json::from_str(&cred_str)
            .map_err(|_| AuthError::Internal("malformed credential record".into()))?;
        let stored_hash = cred["hash"]
            .as_str()
            .ok_or_else(|| AuthError::Internal("malformed credential record".into()))?;

        password::verify(current_password, stored_hash)?;

        let new_hash = password::hash(new_password)?;
        let credential = serde_json::to_string(&json!({ "hash": new_hash }))
            .map_err(|e| AuthError::Internal(e.to_string()))?;

        let mut tx = self.storage.begin().await?;
        tx.update_user_credential(user_id, "password", &credential)
            .await?;
        tx.set_admin_flags(user_id, user.is_admin, false).await?;
        tx.commit().await?;

        Ok(())
    }

    /// Sets a new password without verifying the old one, and forces the
    /// account to change it again on next login.
    #[instrument(skip(self, new_password), fields(%user_id))]
    async fn reset_password_impl(&self, user_id: Uuid, new_password: &str) -> Result<()> {
        validate::password(new_password)?;

        let user = self
            .storage
            .get_user_by_id(user_id)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        let new_hash = password::hash(new_password)?;
        let credential = serde_json::to_string(&json!({ "hash": new_hash }))
            .map_err(|e| AuthError::Internal(e.to_string()))?;

        let mut tx = self.storage.begin().await?;
        tx.update_user_credential(user_id, "password", &credential)
            .await?;
        tx.set_admin_flags(user_id, user.is_admin, true).await?;
        tx.commit().await?;

        Ok(())
    }

    /// Creates the configured default admin account if it doesn't already exist.
    /// No-op otherwise, so it's safe to call on every startup.
    #[instrument(skip(self, password_str), fields(username))]
    pub async fn ensure_default_admin(
        &self,
        username: &str,
        email: &str,
        password_str: &str,
    ) -> Result<()> {
        if self.storage.get_user_by_username(username).await?.is_some() {
            return Ok(());
        }

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
        tx.insert_user_identity(
            Uuid::new_v4(),
            user_id,
            "password",
            &provider_uid,
            &credential,
        )
        .await?;
        tx.set_admin_flags(user_id, true, true).await?;
        tx.commit().await?;

        Ok(())
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

    async fn change_password(
        &self,
        user_id: Uuid,
        current_password: &str,
        new_password: &str,
    ) -> Result<()> {
        self.change_password_impl(user_id, current_password, new_password)
            .await
    }

    async fn reset_password(&self, user_id: Uuid, new_password: &str) -> Result<()> {
        self.reset_password_impl(user_id, new_password).await
    }
}
