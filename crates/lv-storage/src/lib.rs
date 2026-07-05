pub mod error;

use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use lv_core::models::{ApiTokenSummary, AuthSession, Repository, RepoRole, User, Visibility};

pub use error::{Result, StorageError};

/// Reads and single-statement writes. Multi-row writes that must commit
/// atomically go through [`Storage::begin`] instead.
#[async_trait]
pub trait Storage: Send + Sync + 'static {
    async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>>;
    async fn list_users(&self) -> Result<Vec<User>>;

    /// Looks up a user by provider + username-or-email login, returning the
    /// user alongside its stored credential payload for that identity.
    async fn get_user_identity_by_login(
        &self,
        provider: &str,
        login: &str,
    ) -> Result<Option<(User, String)>>;

    async fn insert_api_token(
        &self,
        id: Uuid,
        user_id: Uuid,
        name: &str,
        token_hash: &str,
    ) -> Result<()>;
    async fn find_user_by_token_hash(&self, token_hash: &str) -> Result<Option<User>>;
    async fn list_api_tokens(&self, user_id: Uuid) -> Result<Vec<ApiTokenSummary>>;

    async fn get_repository_by_owner_and_name(
        &self,
        owner_username: &str,
        repo_name: &str,
    ) -> Result<Option<Repository>>;
    async fn get_repo_permission(&self, repo_id: Uuid, user_id: Uuid) -> Result<Option<RepoRole>>;
    /// Deletes the repository row; `repo_permissions` cascades via FK.
    async fn delete_repository(&self, id: Uuid) -> Result<()>;
    async fn list_repositories_by_owner(&self, owner_id: Uuid) -> Result<Vec<Repository>>;
    /// Every repository on the instance, paired with its owner's username. Admin-only use.
    async fn list_repositories(&self) -> Result<Vec<(Repository, String)>>;

    async fn start_auth_session(&self, code: &str, expires_at: OffsetDateTime) -> Result<()>;
    async fn get_auth_session(
        &self,
        code: &str,
        now: OffsetDateTime,
    ) -> Result<Option<AuthSession>>;
    async fn complete_auth_session(
        &self,
        code: &str,
        token: &str,
        user_id: Uuid,
        username: &str,
        expires_at: OffsetDateTime,
    ) -> Result<()>;

    async fn ping(&self) -> Result<()>;

    /// Opens a unit of work for a multi-row write that must commit atomically.
    async fn begin(&self) -> Result<Box<dyn StorageTx>>;
}

/// A single atomic multi-row write. Dropping without calling [`commit`](StorageTx::commit)
/// rolls back.
#[async_trait]
pub trait StorageTx: Send {
    async fn insert_user(&mut self, id: Uuid, username: &str, email: &str) -> Result<()>;
    async fn insert_user_identity(
        &mut self,
        id: Uuid,
        user_id: Uuid,
        provider: &str,
        provider_uid: &str,
        credential_json: &str,
    ) -> Result<()>;

    async fn insert_repository(
        &mut self,
        id: Uuid,
        owner_id: Uuid,
        name: &str,
        visibility: Visibility,
    ) -> Result<()>;
    async fn insert_repo_permission(
        &mut self,
        repo_id: Uuid,
        user_id: Uuid,
        role: RepoRole,
    ) -> Result<()>;

    /// Replaces the stored credential payload for a user's identity with the given provider.
    async fn update_user_credential(
        &mut self,
        user_id: Uuid,
        provider: &str,
        credential_json: &str,
    ) -> Result<()>;
    /// Sets both admin flags to absolute values (not a delta) — callers that only
    /// mean to change one flag must pass through the other's current value.
    async fn set_admin_flags(
        &mut self,
        user_id: Uuid,
        is_admin: bool,
        must_change_password: bool,
    ) -> Result<()>;

    async fn commit(&mut self) -> Result<()>;
}
