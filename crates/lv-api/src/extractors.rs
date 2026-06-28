use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use lv_auth::jwt::JwtConfig;

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
    pub org_id: Option<Uuid>,
}

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = (StatusCode, Json<Value>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer_token(parts).ok_or_else(|| {
            rejection(StatusCode::UNAUTHORIZED, "missing or invalid authorization header")
        })?;

        let jwt = JwtConfig {
            secret: state.config.auth.jwt_secret.clone(),
            ttl_secs: state.config.auth.jwt_ttl_secs,
        };

        let claims = jwt.decode(token).map_err(|e| {
            use lv_auth::error::AuthError;
            let msg = match e {
                AuthError::TokenExpired => "token has expired",
                _ => "invalid token",
            };
            rejection(StatusCode::UNAUTHORIZED, msg)
        })?;

        Ok(AuthenticatedUser {
            user_id: claims.sub,
            org_id: claims.org,
        })
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
    (
        status,
        Json(json!({ "error": message, "code": "unauthorized" })),
    )
}
