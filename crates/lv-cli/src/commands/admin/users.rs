use std::io::Write;

use lv_api_types::admin::AdminUserSummary;
use lv_api_types::auth::{AuthResponse, RegisterRequest};
use tabwriter::TabWriter;

use crate::client::ApiClient;
use crate::pagination;

pub async fn list(
    client: &ApiClient,
    limit: Option<u32>,
    cursor: Option<String>,
) -> anyhow::Result<()> {
    let users =
        pagination::collect::<AdminUserSummary>(client, "/api/v1/admin/users", limit, cursor)
            .await?;

    let mut tw = TabWriter::new(std::io::stdout());
    writeln!(tw, "USERNAME\tEMAIL\tADMIN\tCREATED")?;

    for user in users {
        writeln!(
            tw,
            "{}\t{}\t{}\t{}",
            user.username,
            user.email,
            user.is_admin,
            user.created_at.date()
        )?;
    }

    tw.flush()?;

    Ok(())
}

/// Creates a new user account. Requires the caller to be an admin (the
/// server also allows this when `open_user_creation` is set, but the CLI has
/// no unauthenticated path to it — that's for self-service signup UIs).
pub async fn create(
    client: &ApiClient,
    username: String,
    email: String,
    password: Option<String>,
) -> anyhow::Result<()> {
    let password = match password {
        Some(password) => password,
        None => rpassword::prompt_password("Password: ")?,
    };

    let resp: AuthResponse = client
        .post(
            "/api/v1/auth/register",
            &RegisterRequest {
                username: username.clone(),
                email,
                password,
            },
        )
        .await?;

    println!("Created user {username} ({}).", resp.user_id);

    Ok(())
}

/// TODO: `client.post_no_content("/api/v1/admin/users/{username}/admin", &lv_api_types::admin::SetAdminRequest { is_admin })`.
/// The server refuses to revoke the last remaining admin (409 conflict) — surface that error as-is.
pub async fn set_admin(
    _client: &ApiClient,
    _username: String,
    _is_admin: bool,
) -> anyhow::Result<()> {
    anyhow::bail!("`lorevault admin users set-admin` is not implemented yet")
}

/// TODO: `client.post_no_content("/api/v1/auth/password/reset", &lv_api_types::auth::ResetPasswordRequest { username, new_password })`.
pub async fn reset_password(
    _client: &ApiClient,
    _username: String,
    _new_password: String,
) -> anyhow::Result<()> {
    anyhow::bail!("`lorevault admin users reset-password` is not implemented yet")
}
