use std::io::Write;

use lv_api_types::admin::AdminUserSummary;
use tabwriter::TabWriter;

use crate::client::ApiClient;

/// TODO: `client.get::<Vec<lv_api_types::admin::AdminUserSummary>>("/api/v1/admin/users")`.
pub async fn list(client: &ApiClient) -> anyhow::Result<()> {
    let users = client
        .get::<Vec<AdminUserSummary>>("/api/v1/admin/users")
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
