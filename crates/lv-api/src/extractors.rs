use axum::{
    extract::{FromRequestParts, OptionalFromRequestParts},
    http::{request::Parts, StatusCode},
    Json,
};
use lv_core::models::User;
use serde_json::{json, Value};
use uuid::Uuid;

use lv_auth::jwt;

use crate::state::AppState;

/// Extracts the authenticated user from the `Authorization: Bearer <jwt>` header.
///
/// Use as a route handler parameter to require authentication:
/// ```ignore
/// async fn my_handler(user: AuthenticatedUser, ...) -> impl IntoResponse { ... }
/// ```
/// Returns `401 Unauthorized` if the header is missing, malformed, or the token is expired.
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub is_admin: bool,
}

impl From<User> for AuthenticatedUser {
    fn from(value: User) -> Self {
        AuthenticatedUser {
            user_id: value.id,
            is_admin: value.is_admin,
        }
    }
}

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = (StatusCode, Json<Value>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer_token(parts).ok_or_else(|| {
            rejection(
                StatusCode::UNAUTHORIZED,
                "missing or invalid authorization header",
            )
        })?;

        let claims = jwt::decode(&state.jwt, token).map_err(|e| {
            use lv_auth::error::AuthError;
            let msg = match e {
                AuthError::TokenExpired => "token has expired",
                _ => "invalid token",
            };
            rejection(StatusCode::UNAUTHORIZED, msg)
        })?;

        let user = state
            .storage
            .get_user_by_id(claims.sub)
            .await
            .map_err(|e| rejection(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
            .ok_or_else(|| rejection(StatusCode::UNAUTHORIZED, "user not found"))?;

        Ok(user.into())
    }
}

/// Lets `Option<AuthenticatedUser>` be used as an extractor, e.g. for routes that
/// serve both anonymous and authenticated callers different results (public vs.
/// private repo visibility) rather than rejecting outright when no/invalid
/// credentials are given.
impl OptionalFromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = (StatusCode, Json<Value>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Option<Self>, Self::Rejection> {
        match <Self as FromRequestParts<AppState>>::from_request_parts(parts, state).await {
            Ok(user) => Ok(Some(user)),
            Err(_) => Ok(None),
        }
    }
}

/// Extracts the authenticated user and requires `is_admin`.
///
/// Use as a route handler parameter to restrict a route to admins:
/// ```ignore
/// async fn my_handler(admin: AdminUser, ...) -> impl IntoResponse { ... }
/// ```
/// Returns `401 Unauthorized` under the same conditions as `AuthenticatedUser`,
/// or `403 Forbidden` if the authenticated user is not an admin.
pub struct AdminUser {
    pub user_id: Uuid,
}

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = (StatusCode, Json<Value>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = <AuthenticatedUser as FromRequestParts<AppState>>::from_request_parts(parts, state).await?;
        if !user.is_admin {
            return Err(rejection(StatusCode::FORBIDDEN, "admin privileges required"));
        }
        Ok(AdminUser {
            user_id: user.user_id,
        })
    }
}

/// Lets `Option<AdminUser>` be used as an extractor, e.g. to allow a route
/// gated on some other condition (a config flag) to fall back to "not an
/// admin" rather than rejecting outright when no/invalid credentials are given.
impl OptionalFromRequestParts<AppState> for AdminUser {
    type Rejection = (StatusCode, Json<Value>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Option<Self>, Self::Rejection> {
        match <Self as FromRequestParts<AppState>>::from_request_parts(parts, state).await {
            Ok(admin) => Ok(Some(admin)),
            Err(_) => Ok(None),
        }
    }
}

fn bearer_token(parts: &Parts) -> Option<&str> {
    parts
        .headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}

fn rejection(status: StatusCode, message: &str) -> (StatusCode, Json<Value>) {
    let code = if status == StatusCode::FORBIDDEN {
        "forbidden"
    } else {
        "unauthorized"
    };
    (status, Json(json!({ "error": message, "code": code })))
}
