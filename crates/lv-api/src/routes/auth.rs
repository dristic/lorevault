use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    response::Html,
    Form, Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;

use lv_auth::{jwt, provider::NewUser, token as api_token};

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
                username: req.username.clone(),
                email: req.email,
            },
            json!({ "password": req.password }),
        )
        .await
        .map_err(ApiError::from)?;

    let token = issue_jwt(&state, user_id, &req.username)?;
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

    let username: String = sqlx::query_scalar("SELECT username FROM users WHERE id = ?")
        .bind(user_id.to_string())
        .fetch_one(&state.db)
        .await
        .map_err(|e: sqlx::Error| ApiError::Internal(e.into()))?;

    let token = issue_jwt(&state, user_id, &username)?;
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

    sqlx::query("INSERT INTO api_tokens (id, user_id, name, token_hash) VALUES (?, ?, ?, ?)")
        .bind(Uuid::new_v4().to_string())
        .bind(user.user_id.to_string())
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
pub async fn browser_login_form(Query(params): Query<HashMap<String, String>>) -> Html<String> {
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

    let username: String = match sqlx::query_scalar("SELECT username FROM users WHERE id = ?")
        .bind(user_id.to_string())
        .fetch_one(&state.db)
        .await
    {
        Ok(u) => u,
        Err(_) => return Html(login_form_html(&form.session, Some("Internal error."))),
    };

    let token = match jwt::encode(&state.jwt, user_id, &username) {
        Ok(t) => t,
        Err(_) => return Html(login_form_html(&form.session, Some("Internal error."))),
    };

    // Keep the completed session alive long enough for the CLI to poll it.
    let expires_at = OffsetDateTime::now_utc().unix_timestamp() + 120;
    let _ = sqlx::query(
        "UPDATE auth_sessions SET state = 'complete', token = ?, user_id = ?, username = ?, expires_at = ? WHERE code = ?",
    )
    .bind(&token)
    .bind(user_id.to_string())
    .bind(&username)
    .bind(expires_at)
    .bind(&form.session)
    .execute(&state.db)
    .await;

    Html(login_success_html())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn issue_jwt(state: &AppState, user_id: Uuid, username: &str) -> Result<String> {
    jwt::encode(&state.jwt, user_id, username)
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
  <style>body { font-family: sans-serif; max-width: 400px; margin: 80px auto; padding: 0 1rem; }</style>
</head>
<body>
  <h2>Authorisation successful</h2>
  <p>You can close this window and return to the terminal.</p>
</body>
</html>"#
    .to_string()
}
