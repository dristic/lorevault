use std::{net::SocketAddr, sync::Arc};

use lv_storage::Storage;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use lv_auth::{jwt::JwtConfig, providers::password::PasswordProvider};
use lv_gateway::state::GatewayState;

use lv_api::{config::Settings, routes, state::AppState};
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = Settings::load()?;

    let pool = lv_storage_sqlite::connect(&cfg.database.url).await?;
    sqlx::migrate!("../../migrations").run(&pool).await?;
    let storage: Arc<dyn Storage> = Arc::new(lv_storage_sqlite::SqliteStorage::new(pool));

    let password_provider = PasswordProvider::new(storage.clone());
    password_provider
        .ensure_default_admin(
            &cfg.admin.admin_user,
            &cfg.admin.admin_email,
            &cfg.admin.admin_password,
        )
        .await?;
    let auth = Arc::new(password_provider);

    // Load JWT config — audience is the gRPC hostname so Lore's domain check passes.
    let private_pem = std::fs::read(&cfg.auth.jwt_private_key_pem)?;
    let grpc_hostname = Url::parse(&cfg.server.public_url)?
        .host_str()
        .unwrap_or_default()
        .to_string();
    let mut audience = vec![grpc_hostname];
    audience.extend(cfg.auth.jwt_extra_audience.clone());
    let jwt = Arc::new(JwtConfig::from_rsa_pem(
        cfg.auth.jwt_issuer.clone(),
        audience,
        &private_pem,
        cfg.auth.jwt_ttl_secs,
    )?);

    let app_state = AppState::new(cfg.clone(), storage.clone(), auth, jwt.clone());
    let app = routes::router(app_state);

    let gateway_state = GatewayState::new(
        storage,
        jwt.clone(),
        cfg.server.public_url.clone(),
        cfg.server.web_url.clone(),
    );

    let addr: SocketAddr = cfg.server.bind.parse()?;
    info!(%addr, "REST API listening");

    let grpc_addr: SocketAddr = cfg.server.grpc_bind.parse()?;
    let tls_cert = cfg.server.tls_cert.clone();
    let tls_key = cfg.server.tls_key.clone();
    tokio::spawn(async move {
        if let Err(e) = lv_gateway::server::serve(grpc_addr, gateway_state, tls_cert, tls_key).await
        {
            tracing::error!("gRPC auth service error: {e}");
        }
    });

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
