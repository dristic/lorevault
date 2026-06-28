use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

// ── Users & Orgs ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum OrgRole {
    Owner,
    Admin,
    Member,
}

// ── Repositories ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum OwnerType {
    User,
    Org,
}

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
    pub owner_type: OwnerType,
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
