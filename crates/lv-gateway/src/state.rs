use sqlx::AnyPool;

#[derive(Clone)]
pub struct GatewayState {
    pub db: AnyPool,
    pub jwt_secret: String,
    pub jwt_ttl_secs: i64,
    pub issuer: String,
    pub server_url: String,
    pub web_url: String,
}

impl GatewayState {
    pub fn new(
        db: AnyPool,
        jwt_secret: String,
        jwt_ttl_secs: i64,
        server_url: String,
        web_url: String,
    ) -> Self {
        let issuer = server_url
            .split_once("://")
            .map(|(_, rest)| rest.split(':').next().unwrap_or(rest).to_string())
            .unwrap_or_else(|| server_url.clone());
        Self {
            db,
            jwt_secret,
            jwt_ttl_secs,
            issuer,
            server_url,
            web_url,
        }
    }
}
