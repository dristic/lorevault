use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use lv_auth::{jwt::JwtConfig, password, token as api_token};

use crate::{
    error::{ApiError, Result},
    state::AppState,
};

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: Uuid,
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>> {
    let hash = password::hash(&req.password)
        .map_err(|_| ApiError::BadRequest("password hashing failed".into()))?;

    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash) VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(&req.username)
    .bind(&req.email)
    .bind(&hash)
    .execute(&state.db)
    .await
    .map_err(|e: sqlx::Error| match e {
        sqlx::Error::Database(ref d) if d.constraint() == Some("users_username_key") => {
            ApiError::Conflict("username already taken".into())
        }
        sqlx::Error::Database(ref d) if d.constraint() == Some("users_email_key") => {
            ApiError::Conflict("email already registered".into())
        }
        e => ApiError::Internal(e.into()),
    })?;

    let jwt = JwtConfig {
        secret: state.config.auth.jwt_secret.clone(),
        ttl_secs: state.config.auth.jwt_ttl_secs,
    };
    let token = jwt
        .encode(user_id)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;

    Ok(Json(AuthResponse { token, user_id }))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>> {
    let row: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT id, password_hash FROM users WHERE username = $1",
    )
    .bind(&req.username)
    .fetch_optional(&state.db)
    .await
    .map_err(|e: sqlx::Error| ApiError::Internal(e.into()))?;

    let (user_id, stored_hash) = row.ok_or(ApiError::Unauthorized)?;

    password::verify(&req.password, &stored_hash).map_err(|_| ApiError::Unauthorized)?;

    let jwt = JwtConfig {
        secret: state.config.auth.jwt_secret.clone(),
        ttl_secs: state.config.auth.jwt_ttl_secs,
    };
    let token = jwt
        .encode(user_id)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;

    Ok(Json(AuthResponse { token, user_id }))
}

#[derive(Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct CreateTokenResponse {
    /// Shown once — the raw token value.
    pub token: String,
    pub name: String,
}

pub async fn create_token(
    State(state): State<AppState>,
    Json(req): Json<CreateTokenRequest>,
) -> Result<Json<CreateTokenResponse>> {
    // TODO: extract authenticated user from JWT header
    let user_id = Uuid::nil(); // placeholder until auth extractor is wired

    let (raw, hash) = api_token::generate_api_token();
    sqlx::query(
        "INSERT INTO api_tokens (id, user_id, name, token_hash) VALUES ($1, $2, $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(&req.name)
    .bind(&hash)
    .execute(&state.db)
    .await
    .map_err(|e: sqlx::Error| ApiError::Internal(e.into()))?;

    Ok(Json(CreateTokenResponse {
        token: raw,
        name: req.name,
    }))
}
