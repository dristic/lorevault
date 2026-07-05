use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Mirrors `lv_core::models::Visibility` — duplicated rather than shared so
/// this crate doesn't need `lv-core`'s `sqlx` dependency. `lv-api` converts
/// between the two at the handler boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Public,
    Private,
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Visibility::Public => write!(f, "public"),
            Visibility::Private => write!(f, "private"),
        }
    }
}

/// Mirrors `lv_core::models::RepoRole` — see `Visibility` above for why this
/// is duplicated rather than shared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepoRole {
    Read,
    Write,
    Admin,
}

impl fmt::Display for RepoRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepoRole::Read => write!(f, "read"),
            RepoRole::Write => write!(f, "write"),
            RepoRole::Admin => write!(f, "admin"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub visibility: Visibility,
    pub default_branch: String,
}

/// Body for `POST /api/v1/repos/{owner}/{repo}/users/{username}`. The repo
/// and target user are identified in the URL path (owner/repo/username), not
/// here, so callers never need to know a UUID.
#[derive(Debug, Serialize, Deserialize)]
pub struct SetRepoUserRequest {
    pub role: RepoRole,
}
