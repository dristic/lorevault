use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, Result}, extractors::AuthenticatedUser, state::AppState
};

#[derive(Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
}

pub async fn get_user(
    State(state): State<AppState>,
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
    }))
}
