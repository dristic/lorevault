use std::{net::SocketAddr, sync::Arc};

use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use lv_auth::providers::password::PasswordProvider;

mod config;
mod error;
mod extractors;
mod routes;
mod state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = config::Settings::load()?;

    let db = lv_storage::db::connect(&cfg.database.url, cfg.database.max_connections).await?;
    sqlx::migrate!("../../migrations").run(&db).await?;

    let cache = lv_storage::cache::connect(&cfg.redis.url)?;

    let auth = Arc::new(PasswordProvider::new(db.clone()));

    let state = state::AppState::new(cfg.clone(), db, cache, auth);

    let app = routes::router(state);

    let addr: SocketAddr = cfg.server.bind.parse()?;
    info!(%addr, "REST API listening");

    let grpc_addr: SocketAddr = cfg.server.grpc_bind.parse()?;
    tokio::spawn(async move {
        if let Err(e) = lv_gateway::server::serve(grpc_addr).await {
            tracing::error!("gRPC gateway error: {e}");
        }
    });

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
