use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub bind: String,
    pub grpc_bind: String,
    /// Public URL announced to Lore CLI clients (gRPC endpoint for the auth service).
    pub public_url: String,
    /// Base HTTP URL for the web UI, used to build browser-login redirect URLs.
    pub web_url: String,
    /// Path to the TLS certificate PEM file.
    pub tls_cert: Option<String>,
    /// Path to the TLS private key PEM file.
    pub tls_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// SQLite database URL, e.g. `sqlite:./data/lorevault.db`
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub jwt_issuer: String,
    pub jwt_private_key_pem: String,
    pub jwt_ttl_secs: i64,
    #[serde(default)]
    pub jwt_extra_audience: Vec<String>,
}

impl Settings {
    pub fn load() -> Result<Self, ConfigError> {
        Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name("config/local").required(false))
            .add_source(Environment::with_prefix("LV").separator("__"))
            .build()?
            .try_deserialize()
    }
}
