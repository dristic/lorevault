use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Html,
    Form, Json,
};
use serde::Deserialize;
use serde_json::json;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use lv_api_types::auth::{
    AuthResponse, ChangePasswordRequest, CreateTokenRequest, CreateTokenResponse, LoginRequest, RegisterRequest,
    ResetPasswordRequest,
};
use lv_auth::{jwt, provider::NewUser, token as api_token};

use crate::{
    error::{ApiError, Result},
    extractors::{AdminUser, AuthenticatedUser},
    state::AppState,
};

// ── Register ──────────────────────────────────────────────────────────────────

pub async fn register(
    State(state): State<AppState>,
    admin: Option<AdminUser>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>> {
    if !state.config.admin.open_user_creation && admin.is_none() {
        return Err(ApiError::Forbidden);
    }

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
    Ok(Json(AuthResponse {
        token,
        user_id,
        must_change_password: false,
    }))
}

// ── Change Password ───────────────────────────────────────────────────────────

pub async fn change_password(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<StatusCode> {
    state
        .auth
        .change_password(user.user_id, &req.current_password, &req.new_password)
        .await
        .map_err(ApiError::from)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn reset_password(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<StatusCode> {
    let target = state
        .storage
        .get_user_by_username(&req.username)
        .await?
        .ok_or(ApiError::NotFound)?;

    state
        .auth
        .reset_password(target.id, &req.new_password)
        .await
        .map_err(ApiError::from)?;

    Ok(StatusCode::NO_CONTENT)
}

// ── Login ─────────────────────────────────────────────────────────────────────

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>> {
    let user_id = state
        .auth
        .authenticate(json!({ "login": req.login, "password": req.password }))
        .await
        .map_err(ApiError::from)?;

    let user = state
        .storage
        .get_user_by_id(user_id)
        .await?
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("authenticated user not found")))?;

    let token = issue_jwt(&state, user_id, &user.username)?;
    Ok(Json(AuthResponse {
        token,
        user_id,
        must_change_password: user.must_change_password,
    }))
}

// ── API tokens ────────────────────────────────────────────────────────────────

pub async fn create_token(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<CreateTokenRequest>,
) -> Result<Json<CreateTokenResponse>> {
    let (raw, hash) = api_token::generate_api_token();

    state
        .storage
        .insert_api_token(Uuid::new_v4(), user.user_id, &req.name, &hash)
        .await?;

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

    let username = match state.storage.get_user_by_id(user_id).await {
        Ok(Some(user)) => user.username,
        _ => return Html(login_form_html(&form.session, Some("Internal error."))),
    };

    let token = match jwt::encode(&state.jwt, user_id, &username) {
        Ok(t) => t,
        Err(_) => return Html(login_form_html(&form.session, Some("Internal error."))),
    };

    // Keep the completed session alive long enough for the CLI to poll it.
    let expires_at = OffsetDateTime::now_utc() + Duration::seconds(120);
    let _ = state
        .storage
        .complete_auth_session(&form.session, &token, user_id, &username, expires_at)
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
