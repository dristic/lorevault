use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: Uuid,
    pub must_change_password: bool,
}

/// Self-service: the caller changes their own password, proving they know the current one.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// Admin-driven: resets another user's password without knowing the current one.
/// Forces `must_change_password` on the target account.
#[derive(Debug, Serialize, Deserialize)]
pub struct ResetPasswordRequest {
    pub username: String,
    pub new_password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    /// Username or email address.
    pub login: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTokenResponse {
    /// Shown once — the caller must store this; it cannot be retrieved again.
    pub token: String,
    pub name: String,
}
