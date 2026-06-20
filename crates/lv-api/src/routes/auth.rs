use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use lv_auth::{
    jwt::JwtConfig,
    provider::NewUser,
    token as api_token,
};

use crate::{
    error::{ApiError, Result},
    extractors::AuthenticatedUser,
    state::AppState,
};

// ── Register ──────────────────────────────────────────────────────────────────

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
    let user_id = state
        .auth
        .register(
            NewUser {
                username: req.username,
                email: req.email,
            },
            json!({ "password": req.password }),
        )
        .await
        .map_err(ApiError::from)?;

    let token = issue_jwt(&state, user_id)?;
    Ok(Json(AuthResponse { token, user_id }))
}

// ── Login ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct LoginRequest {
    /// Username or email address.
    pub login: String,
    pub password: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>> {
    let user_id = state
        .auth
        .authenticate(json!({ "login": req.login, "password": req.password }))
        .await
        .map_err(ApiError::from)?;

    let token = issue_jwt(&state, user_id)?;
    Ok(Json(AuthResponse { token, user_id }))
}

// ── API tokens ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct CreateTokenResponse {
    /// Shown once — the caller must store this; it cannot be retrieved again.
    pub token: String,
    pub name: String,
}

pub async fn create_token(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<CreateTokenRequest>,
) -> Result<Json<CreateTokenResponse>> {
    let (raw, hash) = api_token::generate_api_token();

    sqlx::query(
        "INSERT INTO api_tokens (id, user_id, name, token_hash) VALUES ($1, $2, $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(user.user_id)
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

// ── Helpers ───────────────────────────────────────────────────────────────────

fn issue_jwt(state: &AppState, user_id: Uuid) -> Result<String> {
    JwtConfig {
        secret: state.config.auth.jwt_secret.clone(),
        ttl_secs: state.config.auth.jwt_ttl_secs,
    }
    .encode(user_id)
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))
}
