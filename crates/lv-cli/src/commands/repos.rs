use crate::client::ApiClient;

/// TODO: `client.get::<Vec<lv_api_types::users::RepoSummary>>("/api/v1/users/me/repos")`, then print as a table.
pub async fn list(_client: &ApiClient) -> anyhow::Result<()> {
    anyhow::bail!("`lorevault repos` is not implemented yet")
}
