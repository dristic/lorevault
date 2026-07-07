use axum::{
    extract::{Path, Query, State},
    Json,
};

use lv_api_types::pagination::{Page, PageParams};
use lv_api_types::users::{RepoSummary, TokenSummary, UserResponse};
use lv_core::pagination::clamp_limit;

use crate::{
    dto::visibility_to_wire,
    error::{ApiError, Result},
    extractors::{AdminUser, AuthenticatedUser},
    state::AppState,
};

pub async fn get_user(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(username): Path<String>,
) -> Result<Json<UserResponse>> {
    let user = state
        .storage
        .get_user_by_username(&username)
        .await?
        .ok_or(ApiError::NotFound)?;

    Ok(Json(UserResponse {
        id: user.id,
        username: user.username,
        email: user.email,
        is_admin: user.is_admin,
    }))
}

pub async fn list_my_repos(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(page): Query<PageParams>,
) -> Result<Json<Page<RepoSummary>>> {
    let page = state
        .storage
        .list_repositories_by_owner(user.user_id, clamp_limit(page.limit), page.cursor.as_deref())
        .await?;

    Ok(Json(Page {
        items: page
            .items
            .into_iter()
            .map(|r| RepoSummary {
                id: r.id,
                name: r.name,
                description: r.description,
                visibility: visibility_to_wire(r.visibility),
                default_branch: r.default_branch,
            })
            .collect(),
        next_cursor: page.next_cursor,
    }))
}

pub async fn list_my_tokens(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(page): Query<PageParams>,
) -> Result<Json<Page<TokenSummary>>> {
    let page = state
        .storage
        .list_api_tokens(user.user_id, clamp_limit(page.limit), page.cursor.as_deref())
        .await?;

    Ok(Json(Page {
        items: page
            .items
            .into_iter()
            .map(|t| TokenSummary {
                id: t.id,
                name: t.name,
                created_at: t.created_at,
                last_used: t.last_used,
                expires_at: t.expires_at,
            })
            .collect(),
        next_cursor: page.next_cursor,
    }))
}

pub async fn get_me(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<UserResponse>> {
    let user = state
        .storage
        .get_user_by_id(user.user_id)
        .await?
        .ok_or(ApiError::NotFound)?;

    Ok(Json(UserResponse {
        id: user.id,
        username: user.username,
        email: user.email,
        is_admin: user.is_admin,
    }))
}
