use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    response::Html,
    Form, Json,
};
use deadpool_redis::redis::AsyncCommands;
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

// ── Browser-based login (Lore CLI device flow) ────────────────────────────────

/// `GET /login?session=<code>` — serve the HTML login form.
pub async fn browser_login_form(
    Query(params): Query<HashMap<String, String>>,
) -> Html<String> {
    let session = params.get("session").cloned().unwrap_or_default();
    Html(login_form_html(&session, None))
}

#[derive(Deserialize)]
pub struct BrowserLoginForm {
    pub login: String,
    pub password: String,
    pub session: String,
}

/// `POST /login` — authenticate and complete the session so the CLI can poll it.
pub async fn browser_login_submit(
    State(state): State<AppState>,
    Form(form): Form<BrowserLoginForm>,
) -> Html<String> {
    let user_id = match state
        .auth
        .authenticate(json!({ "login": form.login, "password": form.password }))
        .await
    {
        Ok(id) => id,
        Err(_) => return Html(login_form_html(&form.session, Some("Invalid credentials."))),
    };

    let username: String = match sqlx::query_scalar("SELECT username FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.db)
        .await
    {
        Ok(u) => u,
        Err(_) => return Html(login_form_html(&form.session, Some("Internal error."))),
    };

    let issuer = state
        .config
        .server
        .public_url
        .split_once("://")
        .map(|(_, rest)| rest.split(':').next().unwrap_or(rest).to_string())
        .unwrap_or_else(|| state.config.server.public_url.clone());

    let claims = lv_gateway::jwt::new_claims(
        user_id,
        &username,
        &issuer,
        vec![],
        state.config.auth.jwt_ttl_secs,
    );
    let token = match lv_gateway::jwt::encode_claims(&claims, &state.config.auth.jwt_secret) {
        Ok(t) => t,
        Err(_) => return Html(login_form_html(&form.session, Some("Internal error."))),
    };

    let session_data = json!({
        "state": "complete",
        "token": token,
        "user_id": user_id.to_string(),
        "username": username,
        "expires_at": claims.exp,
    })
    .to_string();

    let mut conn = match state.cache.get().await {
        Ok(c) => c,
        Err(_) => return Html(login_form_html(&form.session, Some("Internal error."))),
    };

    let key = format!("auth:session:{}", form.session);
    // Keep the completed session alive long enough for the CLI to poll it.
    let _: () = conn
        .set_ex(&key, &session_data, 120u64)
        .await
        .unwrap_or(());

    Html(login_success_html())
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

fn login_form_html(session: &str, error: Option<&str>) -> String {
    let error_html = match error {
        Some(msg) => format!(r#"<p style="color:red">{msg}</p>"#),
        None => String::new(),
    };
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>LoreVault — Sign in</title>
  <style>
    body {{ font-family: sans-serif; max-width: 400px; margin: 80px auto; padding: 0 1rem; }}
    input {{ display: block; width: 100%; margin: .4rem 0 1rem; padding: .5rem; box-sizing: border-box; }}
    button {{ padding: .5rem 1.5rem; }}
  </style>
</head>
<body>
  <h2>LoreVault</h2>
  <p>Sign in to authorise the Lore CLI.</p>
  {error_html}
  <form method="POST" action="/login">
    <input type="hidden" name="session" value="{session}">
    <label>Username or email
      <input type="text" name="login" autocomplete="username" autofocus>
    </label>
    <label>Password
      <input type="password" name="password" autocomplete="current-password">
    </label>
    <button type="submit">Sign in</button>
  </form>
</body>
</html>"#
    )
}

fn login_success_html() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>LoreVault — Authorised</title>
  <style>body {{ font-family: sans-serif; max-width: 400px; margin: 80px auto; padding: 0 1rem; }}</style>
</head>
<body>
  <h2>Authorisation successful</h2>
  <p>You can close this window and return to the terminal.</p>
</body>
</html>"#
    .to_string()
}
