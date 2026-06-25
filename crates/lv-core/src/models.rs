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
#[sqlx(type_name = "org_role", rename_all = "lowercase")]
pub enum OrgRole {
    Owner,
    Admin,
    Member,
}

// ── Repositories ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "owner_type", rename_all = "lowercase")]
pub enum OwnerType {
    User,
    Org,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "visibility", rename_all = "lowercase")]
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
#[sqlx(type_name = "repo_role", rename_all = "lowercase")]
pub enum RepoRole {
    Admin,
    Write,
    Read,
}

// ── Lore VCS objects ──────────────────────────────────────────────────────────

/// A branch is a mutable pointer to a revision hash (Lore's model).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub id: Uuid,
    pub repo_id: Uuid,
    pub name: String,
    /// Hex-encoded content hash of the HEAD revision.
    pub head_revision_hash: String,
    pub updated_at: OffsetDateTime,
}

/// Immutable revision node in the revision chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Revision {
    pub id: Uuid,
    pub repo_id: Uuid,
    /// Content-addressed hash (hex) of this revision.
    pub hash: String,
    /// Hashes of parent revisions (empty for root).
    pub parent_hashes: Vec<String>,
    pub author_id: Uuid,
    pub message: String,
    pub timestamp: OffsetDateTime,
}

/// Content-addressed storage index entry for a chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: Uuid,
    pub repo_id: Uuid,
    /// Content hash (hex) — the address in CAS.
    pub hash: String,
    pub size_bytes: i64,
    /// Key in object storage (S3/MinIO).
    pub storage_key: String,
}

// ── Locks ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLock {
    pub id: Uuid,
    pub repo_id: Uuid,
    pub path: String,
    pub locked_by_user_id: Uuid,
    pub workspace_id: String,
    pub acquired_at: OffsetDateTime,
}
