use std::net::SocketAddr;

use anyhow::Result;
use tonic::transport::{Identity, Server, ServerTlsConfig};
use tracing::info;

use crate::proto::auth_api::urc_auth_api_server::UrcAuthApiServer;
use crate::services::auth::AuthApiImpl;
use crate::state::GatewayState;

pub async fn serve(
    addr: SocketAddr,
    state: GatewayState,
    tls_cert: Option<String>,
    tls_key: Option<String>,
) -> Result<()> {
    let mut builder = Server::builder();
    if let (Some(cert), Some(key)) = (tls_cert, tls_key) {
        let cert_pem = std::fs::read(&cert)?;
        let key_pem = std::fs::read(&key)?;
        let tls = ServerTlsConfig::new().identity(Identity::from_pem(cert_pem, key_pem));
        builder = builder.tls_config(tls)?;
        info!(%addr, "gRPC auth service listening (TLS)");
    } else {
        info!(%addr, "gRPC auth service listening (plaintext)");
    }

    builder
        .add_service(UrcAuthApiServer::new(AuthApiImpl { state }))
        .serve(addr)
        .await?;

    Ok(())
}
