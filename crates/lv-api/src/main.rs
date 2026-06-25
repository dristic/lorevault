use std::{net::SocketAddr, sync::Arc};

use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use object_store::{aws::AmazonS3Builder, local::LocalFileSystem, ObjectStore};
use lv_auth::providers::password::PasswordProvider;
use lv_gateway::state::GatewayState;
use lv_storage::blob::BlobStore;

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

    let app_state = state::AppState::new(cfg.clone(), db.clone(), cache, auth);
    let app = routes::router(app_state);

    // Build blob store
    let object_store: Arc<dyn ObjectStore> = match cfg.storage.backend.as_str() {
        "s3" => {
            let bucket = cfg.storage.s3_bucket.as_deref().unwrap_or("lorevault");
            let region = cfg.storage.s3_region.as_deref().unwrap_or("us-east-1");
            let mut builder = AmazonS3Builder::new()
                .with_bucket_name(bucket)
                .with_region(region);
            if let Some(endpoint) = &cfg.storage.s3_endpoint {
                builder = builder.with_endpoint(endpoint).with_allow_http(true);
            }
            Arc::new(builder.build()?)
        }
        _ => {
            let path = cfg.storage.local_path.as_deref().unwrap_or("/tmp/lorevault-blobs");
            std::fs::create_dir_all(path)?;
            Arc::new(LocalFileSystem::new_with_prefix(path)?)
        }
    };
    let blob = Arc::new(BlobStore::new(object_store));

    let gateway_state = GatewayState::new(
        db,
        blob,
        cfg.auth.jwt_secret.clone(),
        cfg.auth.jwt_ttl_secs,
        cfg.server.public_url.clone(),
    );

    let addr: SocketAddr = cfg.server.bind.parse()?;
    info!(%addr, "REST API listening");

    let grpc_addr: SocketAddr = cfg.server.grpc_bind.parse()?;
    let tls_cert = cfg.server.tls_cert.clone();
    let tls_key = cfg.server.tls_key.clone();
    tokio::spawn(async move {
        if let Err(e) = lv_gateway::server::serve(grpc_addr, gateway_state, tls_cert, tls_key).await {
            tracing::error!("gRPC gateway error: {e}");
        }
    });

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
