use lv_api_types::auth::{AuthResponse, LoginRequest};

use crate::{client::ApiClient, config::CliConfig};

pub async fn run(server: String, user: String, password: Option<String>) -> anyhow::Result<()> {
    let password = match password {
        Some(password) => password,
        None => rpassword::prompt_password("Password: ")?,
    };

    let client = ApiClient::new(server.clone(), None);
    let resp: AuthResponse = client
        .post_public(
            "/api/v1/auth/login",
            &LoginRequest { login: user.clone(), password },
        )
        .await?;

    let mut cfg = CliConfig::load()?;
    cfg.server = Some(server);
    cfg.token = Some(resp.token);
    cfg.save()?;

    println!("Logged in as {user}.");
    if resp.must_change_password {
        println!("Your password must be changed — run `lorevault passwd`.");
    }
    Ok(())
}
