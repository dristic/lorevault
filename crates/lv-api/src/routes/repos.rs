use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use lv_api_types::repos::{RepoResponse, SetRepoUserRequest};
use lv_core::models::{RepoRole, Repository, Visibility};
use uuid::Uuid;

use crate::{
    dto::{repo_role_from_wire, visibility_to_wire},
    error::{ApiError, Result},
    extractors::AuthenticatedUser,
    state::AppState,
};

pub async fn get_repo(
    State(state): State<AppState>,
    user: Option<AuthenticatedUser>,
    Path((owner, repo)): Path<(String, String)>,
) -> Result<Json<RepoResponse>> {
    let repo = state
        .storage
        .get_repository_by_owner_and_name(&owner, &repo)
        .await?
        .ok_or(ApiError::NotFound)?;

    if repo.visibility == Visibility::Private {
        let can_read = match &user {
            Some(u) if u.is_admin => true,
            Some(u) => state
                .storage
                .get_repo_permission(repo.id, u.user_id)
                .await?
                .is_some(),
            None => false,
        };
        // 404 rather than 403 so a private repo's existence isn't leaked to non-viewers.
        if !can_read {
            return Err(ApiError::NotFound);
        }
    }

    Ok(Json(RepoResponse {
        id: repo.id,
        name: repo.name,
        description: repo.description,
        visibility: visibility_to_wire(repo.visibility),
        default_branch: repo.default_branch,
    }))
}

/// Grants `username` `role` on `owner/repo`, or changes their existing role.
/// Only callable by a repo admin (or an instance admin).
pub async fn add_repo_user(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((owner, repo, username)): Path<(String, String, String)>,
    Json(req): Json<SetRepoUserRequest>,
) -> Result<StatusCode> {
    let repo = resolve_repo(&state, &owner, &repo).await?;
    require_repo_admin(&state, &user, repo.id).await?;

    let target = state
        .storage
        .get_user_by_username(&username)
        .await?
        .ok_or(ApiError::NotFound)?;

    state
        .storage
        .set_repo_permission(repo.id, target.id, repo_role_from_wire(req.role))
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Revokes `username`'s access to `owner/repo`. Only callable by a repo admin
/// (or an instance admin).
pub async fn remove_repo_user(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((owner, repo, username)): Path<(String, String, String)>,
) -> Result<StatusCode> {
    let repo = resolve_repo(&state, &owner, &repo).await?;
    require_repo_admin(&state, &user, repo.id).await?;

    let target = state
        .storage
        .get_user_by_username(&username)
        .await?
        .ok_or(ApiError::NotFound)?;

    state
        .storage
        .remove_repo_permission(repo.id, target.id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn resolve_repo(state: &AppState, owner: &str, repo: &str) -> Result<Repository> {
    state
        .storage
        .get_repository_by_owner_and_name(owner, repo)
        .await?
        .ok_or(ApiError::NotFound)
}

async fn require_repo_admin(
    state: &AppState,
    user: &AuthenticatedUser,
    repo_id: Uuid,
) -> Result<()> {
    if user.is_admin {
        return Ok(());
    }
    match state
        .storage
        .get_repo_permission(repo_id, user.user_id)
        .await?
    {
        Some(RepoRole::Admin) => Ok(()),
        _ => Err(ApiError::Forbidden),
    }
}
