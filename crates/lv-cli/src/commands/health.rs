use serde_json::Value;

use crate::{client::ApiClient, config::CliConfig};

/// Checks `/healthz` on the given server, or the currently logged-in server
/// if none is given. Doesn't require being logged in — `/healthz` is public.
pub async fn run(server: Option<String>) -> anyhow::Result<()> {
    let server = match server {
        Some(server) => server,
        None => CliConfig::load()?.server.ok_or_else(|| {
            anyhow::anyhow!("no server specified — pass one or run `lorevault login` first")
        })?,
    };

    let client = ApiClient::new(server.clone(), None);
    let (status, body) = client.get_public_raw("/healthz").await?;

    let reported_status = body
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    println!("{server}: {reported_status} (HTTP {status})");
    if let Some(detail) = body.get("detail").and_then(Value::as_str) {
        println!("  {detail}");
    }

    if status != 200 {
        anyhow::bail!("server is not healthy");
    }
    Ok(())
}
