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
    let row: Option<(String, String, Option<String>, String, String)> = sqlx::query_as(
        r#"SELECT r.id, r.name, r.description, r.visibility, r.default_branch
           FROM repositories r
           JOIN users u ON r.owner_id = u.id
           WHERE u.username = ? AND r.name = ?"#,
    )
    .bind(&owner)
    .bind(&repo)
    .fetch_optional(&state.db)
    .await
    .map_err(|e: sqlx::Error| ApiError::Internal(e.into()))?;

    let (id_str, name, description, vis_str, default_branch) = row.ok_or(ApiError::NotFound)?;
    let id = Uuid::parse_str(&id_str).map_err(|_| ApiError::Internal(anyhow::anyhow!("malformed id")))?;
    let visibility = match vis_str.as_str() {
        "public" => Visibility::Public,
        _ => Visibility::Private,
    };

    Ok(Json(RepoResponse {
        id,
        name,
        description,
        visibility,
        default_branch,
    }))
}
