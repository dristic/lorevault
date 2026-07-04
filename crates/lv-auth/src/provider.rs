use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::error::{AuthError, Result};

/// Common user information required when creating a new account.
/// Provider-specific credentials (passwords, OAuth tokens, etc.) are passed
/// separately as an untyped `Value` so each provider can define its own shape.
pub struct NewUser {
    pub username: String,
    pub email: String,
}

/// Extensible authentication provider interface.
///
/// Each implementation owns its credential storage and verification logic.
/// The `credentials` parameter in both methods is a provider-specific JSON
/// object documented by each concrete type.
///
/// # Implementing a new provider
///
/// 1. Add the provider struct + `impl AuthProvider` in a new `providers/<name>.rs`.
/// 2. Add a `user_identities` row with `provider = "<name>"` on first auth.
/// 3. Register the provider in `AppState` (currently a single `Arc<dyn AuthProvider>`;
///    extend to a map keyed by provider name when multiple are needed).
#[async_trait]
pub trait AuthProvider: Send + Sync + 'static {
    /// Stable identifier for this provider, e.g. `"password"`, `"github"`.
    fn name(&self) -> &'static str;

    /// Create a new user account linked to this provider.
    ///
    /// Returns the new user's ID on success.
    /// Providers that don't support explicit registration (e.g. OAuth, where
    /// the account is created on first login) should return `Err(AuthError::NotSupported)`.
    async fn register(&self, user: NewUser, credentials: Value) -> Result<Uuid>;

    /// Verify the supplied credentials and return the authenticated user's ID.
    async fn authenticate(&self, credentials: Value) -> Result<Uuid>;

    /// Self-service password change: verifies `current_password` before replacing it.
    /// Providers that don't manage passwords (e.g. OAuth) should return `Err(AuthError::NotSupported)`.
    async fn change_password(
        &self,
        user_id: Uuid,
        current_password: &str,
        new_password: &str,
    ) -> Result<()> {
        let _ = (user_id, current_password, new_password);
        Err(AuthError::NotSupported)
    }

    /// Admin-driven reset: sets a new password without verifying the old one,
    /// and flags the account so the user must change it again on next login.
    /// Providers that don't manage passwords (e.g. OAuth) should return `Err(AuthError::NotSupported)`.
    async fn reset_password(&self, user_id: Uuid, new_password: &str) -> Result<()> {
        let _ = (user_id, new_password);
        Err(AuthError::NotSupported)
    }
}
