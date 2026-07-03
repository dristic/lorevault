use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use lv_core::models::Visibility;

use crate::{
    error::{ApiError, Result},
    state::AppState,
};

#[derive(Serialize)]
pub struct RepoResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub visibility: Visibility,
    pub default_branch: String,
}

pub async fn get_repo(
    State(state): State<AppState>,
    Path((owner, repo)): Path<(String, String)>,
) -> Result<Json<RepoResponse>> {
    let repo = state
        .storage
        .get_repository_by_owner_and_name(&owner, &repo)
        .await?
        .ok_or(ApiError::NotFound)?;

    Ok(Json(RepoResponse {
        id: repo.id,
        name: repo.name,
        description: repo.description,
        visibility: repo.visibility,
        default_branch: repo.default_branch,
    }))
}
