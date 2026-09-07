use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use tracing::debug;

use crate::error::{CliError, Result};

/// Thin REST client for the lv-api HTTP surface. Deliberately talks JSON over
/// the wire rather than linking lv-core/lv-storage — the CLI is just another
/// API consumer, same as the web frontend would be.
pub struct ApiClient {
    base_url: String,
    token: Option<String>,
    http: reqwest::Client,
}

// `post` and `post_no_content` are unused until the stubbed commands in
// `commands/` are filled in — they're the client-side half of that work.
impl ApiClient {
    pub fn new(base_url: String, token: Option<String>) -> Self {
        Self {
            base_url,
            token,
            http: reqwest::Client::new(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> Result<reqwest::RequestBuilder> {
        let token = self.token.as_deref().ok_or(CliError::NotLoggedIn)?;
        Ok(req.bearer_auth(token))
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = self.url(path);
        debug!(method = "GET", %url, "sending request");
        let req = self.auth(self.http.get(&url))?;
        Self::handle(req.send().await?).await
    }

    /// `GET` a single page of a paginated list endpoint.
    pub async fn get_page<T: DeserializeOwned>(
        &self,
        path: &str,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<lv_api_types::pagination::Page<T>> {
        let url = self.url(path);
        debug!(method = "GET", %url, ?limit, ?cursor, "sending request");
        let mut query = Vec::new();
        if let Some(limit) = limit {
            query.push(("limit", limit.to_string()));
        }
        if let Some(cursor) = cursor {
            query.push(("cursor", cursor.to_string()));
        }
        let req = self.auth(self.http.get(&url))?.query(&query);
        Self::handle(req.send().await?).await
    }

    pub async fn post<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> Result<T> {
        let url = self.url(path);
        debug!(method = "POST", %url, "sending request");
        let req = self.auth(self.http.post(&url).json(body))?;
        Self::handle(req.send().await?).await
    }

    /// For endpoints that return `204 No Content` on success (e.g. password
    /// reset, admin-flag changes) rather than a JSON body.
    pub async fn post_no_content<B: Serialize>(&self, path: &str, body: &B) -> Result<()> {
        let url = self.url(path);
        debug!(method = "POST", %url, "sending request");
        let req = self.auth(self.http.post(&url).json(body))?;
        let resp = req.send().await?;
        debug!(status = %resp.status(), "received response");
        if resp.status().is_success() {
            return Ok(());
        }
        Err(Self::api_error(resp).await)
    }

    /// `DELETE` returning `204 No Content` on success (e.g. revoking a repo
    /// permission).
    pub async fn delete_no_content(&self, path: &str) -> Result<()> {
        let url = self.url(path);
        debug!(method = "DELETE", %url, "sending request");
        let req = self.auth(self.http.delete(&url))?;
        let resp = req.send().await?;
        debug!(status = %resp.status(), "received response");
        if resp.status().is_success() {
            return Ok(());
        }
        Err(Self::api_error(resp).await)
    }

    /// `POST` without auth (login, register-when-open) — caller supplies the
    /// body and gets the raw JSON response back.
    pub async fn post_public<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = self.url(path);
        debug!(method = "POST", %url, "sending request");
        let resp = self.http.post(&url).json(body).send().await?;
        Self::handle(resp).await
    }

    /// `GET` without auth, returning the status code alongside the body
    /// regardless of success — for endpoints like `/healthz` that use a
    /// non-2xx status as a meaningful result rather than an error envelope.
    pub async fn get_public_raw(&self, path: &str) -> Result<(u16, Value)> {
        let url = self.url(path);
        debug!(method = "GET", %url, "sending request");
        let resp = self.http.get(&url).send().await?;
        let status = resp.status().as_u16();
        debug!(status, "received response");
        let body = resp.json::<Value>().await.unwrap_or(Value::Null);
        Ok((status, body))
    }

    async fn handle<T: DeserializeOwned>(resp: reqwest::Response) -> Result<T> {
        debug!(status = %resp.status(), "received response");
        if resp.status().is_success() {
            return Ok(resp.json().await?);
        }
        Err(Self::api_error(resp).await)
    }

    async fn api_error(resp: reqwest::Response) -> CliError {
        let status = resp.status().as_u16();
        let message = match resp.json::<Value>().await {
            Ok(body) => body
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
                .to_string(),
            Err(_) => "unknown error".to_string(),
        };
        CliError::Api { status, message }
    }
}
