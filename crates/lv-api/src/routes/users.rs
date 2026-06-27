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
    let row: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, username FROM users WHERE username = $1")
            .bind(&username)
            .fetch_optional(&state.db)
            .await
            .map_err(|e: sqlx::Error| ApiError::Internal(e.into()))?;

    let (id, username) = row.ok_or(ApiError::NotFound)?;
    Ok(Json(UserResponse { id, username, email: String::new() }))
}

pub async fn get_me(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<UserResponse>> {
    let row: Option<(Uuid, String, String)> =
        sqlx::query_as("SELECT id, username, email FROM users WHERE id = $1")
            .bind(user.user_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e: sqlx::Error| ApiError::Internal(e.into()))?;

    let (id, username, email) = row.ok_or(ApiError::NotFound)?;
    Ok(Json(UserResponse { id, username, email }))
}
