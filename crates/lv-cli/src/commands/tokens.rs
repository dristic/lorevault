use crate::client::ApiClient;

/// TODO: `client.get::<Vec<lv_api_types::users::TokenSummary>>("/api/v1/users/me/tokens")`.
pub async fn list(_client: &ApiClient) -> anyhow::Result<()> {
    anyhow::bail!("`lorevault tokens list` is not implemented yet")
}

/// TODO: `client.post::<_, lv_api_types::auth::CreateTokenResponse>("/api/v1/auth/tokens", &lv_api_types::auth::CreateTokenRequest { name })`.
/// The raw token is only ever shown here — print it clearly and remind the user it can't be retrieved again.
pub async fn create(_client: &ApiClient, _name: String) -> anyhow::Result<()> {
    anyhow::bail!("`lorevault tokens create` is not implemented yet")
}
