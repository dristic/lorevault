use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use lv_api_types::admin::{AdminRepoSummary, AdminUserSummary, SetAdminRequest};

use crate::{
    dto::visibility_to_wire,
    error::{ApiError, Result},
    extractors::AdminUser,
    state::AppState,
};

pub async fn list_users(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<Vec<AdminUserSummary>>> {
    let users = state.storage.list_users().await?;

    Ok(Json(
        users
            .into_iter()
            .map(|u| AdminUserSummary {
                id: u.id,
                username: u.username,
                email: u.email,
                is_admin: u.is_admin,
                must_change_password: u.must_change_password,
                created_at: u.created_at,
            })
            .collect(),
    ))
}

pub async fn list_repos(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<Vec<AdminRepoSummary>>> {
    let repos = state.storage.list_repositories().await?;

    Ok(Json(
        repos
            .into_iter()
            .map(|(r, owner)| AdminRepoSummary {
                id: r.id,
                owner,
                name: r.name,
                description: r.description,
                visibility: visibility_to_wire(r.visibility),
                default_branch: r.default_branch,
            })
            .collect(),
    ))
}

/// Grants or revokes admin privileges for another user. Refuses to revoke the
/// last remaining admin, since that would lock every admin out of the instance.
pub async fn set_admin(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(username): Path<String>,
    Json(req): Json<SetAdminRequest>,
) -> Result<StatusCode> {
    let target = state
        .storage
        .get_user_by_username(&username)
        .await?
        .ok_or(ApiError::NotFound)?;

    if !req.is_admin && target.is_admin {
        let remaining_admins = state
            .storage
            .list_users()
            .await?
            .iter()
            .filter(|u| u.is_admin)
            .count();
        if remaining_admins <= 1 {
            return Err(ApiError::Conflict(
                "cannot revoke the last remaining admin".into(),
            ));
        }
    }

    let mut tx = state.storage.begin().await?;
    tx.set_admin_flags(target.id, req.is_admin, target.must_change_password)
        .await?;
    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}
