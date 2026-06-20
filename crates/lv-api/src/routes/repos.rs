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
           VALUES ($1, $2::owner_type, $3, $4, $5, $6::visibility, 'main')"#,
    )
    .bind(id)
    .bind(owner_type_str)
    .bind(req.owner_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(vis_str)
    .execute(&state.db)
    .await
    .map_err(|e: sqlx::Error| match e {
        sqlx::Error::Database(ref d) if d.constraint() == Some("repositories_owner_id_name_key") => {
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
    // Resolve by username (user-owned) or org slug (org-owned)
    let row: Option<(Uuid, String, Option<String>, String, String)> = sqlx::query_as(
        r#"SELECT r.id, r.name, r.description, r.visibility::text, r.default_branch
           FROM repositories r
           JOIN users u ON r.owner_id = u.id AND r.owner_type = 'user'::owner_type
           WHERE u.username = $1 AND r.name = $2
           UNION ALL
           SELECT r.id, r.name, r.description, r.visibility::text, r.default_branch
           FROM repositories r
           JOIN organizations o ON r.owner_id = o.id AND r.owner_type = 'org'::owner_type
           WHERE o.slug = $1 AND r.name = $2"#,
    )
    .bind(&owner)
    .bind(&repo)
    .fetch_optional(&state.db)
    .await
    .map_err(|e: sqlx::Error| ApiError::Internal(e.into()))?;

    let (id, name, description, vis_str, default_branch) = row.ok_or(ApiError::NotFound)?;
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
