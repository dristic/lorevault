use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, Result},
    state::AppState,
};

#[derive(Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
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
    Ok(Json(UserResponse { id, username }))
}
