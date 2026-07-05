use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::repos::Visibility;

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminUserSummary {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub is_admin: bool,
    pub must_change_password: bool,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminRepoSummary {
    pub id: Uuid,
    pub owner: String,
    pub name: String,
    pub description: Option<String>,
    pub visibility: Visibility,
    pub default_branch: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetAdminRequest {
    pub is_admin: bool,
}
