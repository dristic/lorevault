use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

// ── Users ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub is_admin: bool,
    pub must_change_password: bool,
    pub created_at: OffsetDateTime,
}

/// A user's own API token, without the hash — returned by self-service listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiTokenSummary {
    pub id: Uuid,
    pub name: String,
    pub created_at: OffsetDateTime,
    pub last_used: Option<OffsetDateTime>,
    pub expires_at: Option<OffsetDateTime>,
}

// ── Repositories ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub visibility: Visibility,
    pub default_branch: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum RepoRole {
    Admin,
    Write,
    Read,
}

// ── Auth sessions (Lore CLI device flow) ───────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum AuthSessionState {
    Pending,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub code: String,
    pub state: AuthSessionState,
    pub token: Option<String>,
    pub user_id: Option<Uuid>,
    pub username: Option<String>,
    pub expires_at: OffsetDateTime,
}
