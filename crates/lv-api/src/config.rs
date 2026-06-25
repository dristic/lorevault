use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub auth: AuthConfig,
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub bind: String,
    pub grpc_bind: String,
    /// Public URL announced to Lore CLI clients via EnvironmentGet (gRPC endpoint).
    pub public_url: String,
    /// Base HTTP URL for the web UI, used to build browser-login redirect URLs.
    pub web_url: String,
    /// Path to the TLS certificate PEM file. When set (along with `tls_key`),
    /// the gRPC gateway serves over TLS.
    pub tls_cert: Option<String>,
    /// Path to the TLS private key PEM file.
    pub tls_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_ttl_secs: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    /// "local" (default) or "s3"
    pub backend: String,
    /// Root directory for the local filesystem backend.
    pub local_path: Option<String>,
    /// S3 bucket name (required when backend = "s3").
    pub s3_bucket: Option<String>,
    /// AWS region (required when backend = "s3").
    pub s3_region: Option<String>,
    /// Override endpoint URL for S3-compatible services (optional).
    pub s3_endpoint: Option<String>,
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
