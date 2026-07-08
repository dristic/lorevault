use std::io::Write;

use lv_api_types::{
    auth::{CreateTokenRequest, CreateTokenResponse},
    users::TokenSummary,
};
use tabwriter::TabWriter;

use crate::client::ApiClient;
use crate::pagination;

pub async fn list(
    client: &ApiClient,
    limit: Option<u32>,
    cursor: Option<String>,
) -> anyhow::Result<()> {
    let tokens =
        pagination::collect::<TokenSummary>(client, "/api/v1/users/me/tokens", limit, cursor)
            .await?;

    let mut tw = TabWriter::new(std::io::stdout());
    writeln!(tw, "NAME\tCREATED\tLAST USED\tEXPIRES")?;

    for token in tokens {
        let last_used = token
            .last_used
            .map(|d| d.date().to_string())
            .unwrap_or_else(|| "none".to_string());

        let expires_at = token
            .expires_at
            .map(|d| d.date().to_string())
            .unwrap_or_else(|| "none".to_string());

        writeln!(
            tw,
            "{}\t{}\t{}\t{}",
            token.name,
            token.created_at.date(),
            last_used,
            expires_at
        )?;
    }

    tw.flush()?;

    Ok(())
}

pub async fn create(client: &ApiClient, name: String) -> anyhow::Result<()> {
    let response = client
        .post::<_, CreateTokenResponse>("/api/v1/auth/tokens", &CreateTokenRequest { name })
        .await?;

    println!("Created token {}", response.name);
    println!("Save your token since it is only printed once and can't be retrieved again");
    println!("{}", response.token);

    Ok(())
}
