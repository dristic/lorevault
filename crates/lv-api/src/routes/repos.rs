use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use lv_core::models::{OwnerType, Visibility};

use crate::{
    error::{ApiError, Result},
    state::AppState,
};

#[derive(Deserialize)]
pub struct CreateRepoRequest {
    pub owner_id: Uuid,
    pub owner_type: OwnerType,
    pub name: String,
    pub description: Option<String>,
    pub visibility: Visibility,
}

#[derive(Serialize)]
pub struct RepoResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub visibility: Visibility,
    pub default_branch: String,
}

#[derive(Serialize)]
pub struct DeleteRepoResponse {
    pub id: Uuid,
}

pub async fn create_repo(
    State(state): State<AppState>,
    Json(req): Json<CreateRepoRequest>,
) -> Result<Json<RepoResponse>> {
    let id = Uuid::new_v4();
    let owner_type_str = match req.owner_type {
        OwnerType::User => "user",
        OwnerType::Org => "org",
    };
    let vis_str = match req.visibility {
        Visibility::Public => "public",
        Visibility::Private => "private",
    };

    sqlx::query(
        r#"INSERT INTO repositories (id, owner_type, owner_id, name, description, visibility, default_branch)
           VALUES (?, ?, ?, ?, ?, ?, 'main')"#,
    )
    .bind(id.to_string())
    .bind(owner_type_str)
    .bind(req.owner_id.to_string())
    .bind(&req.name)
    .bind(&req.description)
    .bind(vis_str)
    .execute(&state.db)
    .await
    .map_err(|e: sqlx::Error| match e {
        sqlx::Error::Database(ref d) if d.is_unique_violation() => {
            ApiError::Conflict(format!("repository '{}' already exists for this owner", req.name))
        }
        e => ApiError::Internal(e.into()),
    })?;

    Ok(Json(RepoResponse {
        id,
        name: req.name,
        description: req.description,
        visibility: req.visibility,
        default_branch: "main".to_string(),
    }))
}

pub async fn get_repo(
    State(state): State<AppState>,
    Path((owner, repo)): Path<(String, String)>,
) -> Result<Json<RepoResponse>> {
    let row: Option<(String, String, Option<String>, String, String)> = sqlx::query_as(
        r#"SELECT r.id, r.name, r.description, r.visibility, r.default_branch
           FROM repositories r
           JOIN users u ON r.owner_id = u.id AND r.owner_type = 'user'
           WHERE u.username = ? AND r.name = ?
           UNION ALL
           SELECT r.id, r.name, r.description, r.visibility, r.default_branch
           FROM repositories r
           JOIN organizations o ON r.owner_id = o.id AND r.owner_type = 'org'
           WHERE o.slug = ? AND r.name = ?"#,
    )
    .bind(&owner)
    .bind(&repo)
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

pub async fn delete_repo(
    State(state): State<AppState>,
    Path((owner, repo)): Path<(String, String)>,
) -> Result<Json<DeleteRepoResponse>> {
    let id_str: Option<String> = sqlx::query_scalar(
        r#"SELECT r.id
           FROM repositories r
           JOIN users u ON r.owner_id = u.id AND r.owner_type = 'user'
           WHERE u.username = ? AND r.name = ?
           UNION ALL
           SELECT r.id
           FROM repositories r
           JOIN organizations o ON r.owner_id = o.id AND r.owner_type = 'org'
           WHERE o.slug = ? AND r.name = ?"#,
    )
    .bind(&owner)
    .bind(&repo)
    .bind(&owner)
    .bind(&repo)
    .fetch_optional(&state.db)
    .await
    .map_err(|e: sqlx::Error| ApiError::Internal(e.into()))?;

    let id_str = id_str.ok_or(ApiError::NotFound)?;
    let id = Uuid::parse_str(&id_str).map_err(|_| ApiError::Internal(anyhow::anyhow!("malformed id")))?;

    sqlx::query("DELETE FROM repositories WHERE id = ?")
        .bind(id.to_string())
        .execute(&state.db)
        .await
        .map_err(|e: sqlx::Error| ApiError::Internal(e.into()))?;

    Ok(Json(DeleteRepoResponse { id }))
}
