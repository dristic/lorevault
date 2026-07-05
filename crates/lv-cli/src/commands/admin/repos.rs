use crate::client::ApiClient;

/// TODO: `client.get::<Vec<lv_api_types::admin::AdminRepoSummary>>("/api/v1/admin/repos")`.
pub async fn list(_client: &ApiClient) -> anyhow::Result<()> {
    anyhow::bail!("`lorevault admin repos list` is not implemented yet")
}
