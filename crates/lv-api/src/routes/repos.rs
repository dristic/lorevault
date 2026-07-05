use axum::{
    extract::{Path, State},
    Json,
};

use lv_api_types::repos::RepoResponse;
use lv_core::models::Visibility;

use crate::{
    dto::visibility_to_wire,
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
