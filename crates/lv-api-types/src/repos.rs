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

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub visibility: Visibility,
    pub default_branch: String,
}
