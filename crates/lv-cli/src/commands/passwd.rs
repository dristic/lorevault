use lv_api_types::auth::ChangePasswordRequest;
use serde_json::json;

use crate::client::ApiClient;

pub async fn run(client: &ApiClient) -> anyhow::Result<()> {
    let current_password = rpassword::prompt_password("Current Password: ")?;
    let new_password = rpassword::prompt_password("New Password: ")?;
    let confirm_password = rpassword::prompt_password("Confirm password: ")?;

    if new_password != confirm_password {
        return Err(anyhow::anyhow!("New and confirm passwords did not match."));
    }

    client
        .post_no_content(
            "/api/v1/auth/password",
            &json!(ChangePasswordRequest {
                current_password,
                new_password,
            }),
        )
        .await?;

    println!("Password changed successfully.");

    Ok(())
}
