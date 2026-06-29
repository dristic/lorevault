use std::{net::SocketAddr, sync::Arc};

use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use lv_auth::{jwt::JwtConfig, providers::password::PasswordProvider};
use lv_gateway::state::GatewayState;

use lv_api::{config::Settings, routes, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = Settings::load()?;

    let db = lv_storage::db::connect(&cfg.database.url).await?;
    sqlx::migrate!("../../migrations").run(&db).await?;

    let auth = Arc::new(PasswordProvider::new(db.clone()));

    // Load JWT config
    let private_pem = std::fs::read(&cfg.auth.jwt_private_key_pem)?;
    let jwt = Arc::new(JwtConfig::from_rsa_pem(
        cfg.auth.jwt_issuer.clone(),
        &private_pem,
        cfg.auth.jwt_ttl_secs,
    )?);

    let app_state = AppState::new(cfg.clone(), db.clone(), auth, jwt);
    let app = routes::router(app_state);

    let gateway_state = GatewayState::new(
        db,
        String::from("FIXME: no secret key"),
        cfg.auth.jwt_ttl_secs,
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
