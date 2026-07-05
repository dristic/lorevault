use std::net::SocketAddr;

use anyhow::Result;
use tonic::transport::{Identity, Server, ServerTlsConfig};
use tracing::info;

use crate::proto::auth_api::urc_auth_api_server::UrcAuthApiServer;
use crate::proto::lore_environment_v1::environment_service_server::EnvironmentServiceServer as EnvironmentServiceServerV1;
use crate::proto::rebac::rebac_api_server::RebacApiServer;
use crate::proto::urc::rpc::environment_service_server::EnvironmentServiceServer;
use crate::services::auth::AuthApiImpl;
use crate::services::environment::EnvironmentServiceImpl;
use crate::services::rebac::RebacApiImpl;
use crate::state::GatewayState;

pub async fn serve(
    addr: SocketAddr,
    state: GatewayState,
    tls_cert: Option<String>,
    tls_key: Option<String>,
) -> Result<()> {
    let mut builder = Server::builder();
    if let (Some(cert), Some(key)) = (
        tls_cert.filter(|s| !s.is_empty()),
        tls_key.filter(|s| !s.is_empty()),
    ) {
        let cert_pem = std::fs::read(&cert)?;
        let key_pem = std::fs::read(&key)?;
        let tls = ServerTlsConfig::new().identity(Identity::from_pem(cert_pem, key_pem));
        builder = builder.tls_config(tls)?;
        info!(%addr, "gRPC auth service listening (TLS)");
    } else {
        info!(%addr, "gRPC auth service listening (plaintext)");
    }

    let env_svc = EnvironmentServiceImpl::new(&state);

    builder
        .add_service(UrcAuthApiServer::new(AuthApiImpl {
            state: state.clone(),
        }))
        .add_service(EnvironmentServiceServer::new(env_svc.clone()))
        .add_service(EnvironmentServiceServerV1::new(env_svc))
        .add_service(RebacApiServer::new(RebacApiImpl {
            state: state.clone(),
        }))
        .serve(addr)
        .await?;

    Ok(())
}
