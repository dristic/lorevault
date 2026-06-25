use std::net::SocketAddr;

use tonic::transport::{Identity, Server, ServerTlsConfig};
use tracing::info;

use crate::proto::{
    auth_api::urc_auth_api_server::UrcAuthApiServer,
    environment::environment_service_server::EnvironmentServiceServer,
    lock::lock_service_server::LockServiceServer,
    repository::repository_service_server::RepositoryServiceServer,
    revision::revision_service_server::RevisionServiceServer,
    storage::storage_service_server::StorageServiceServer,
};
use crate::services::{
    auth::AuthApiImpl,
    environment::EnvironmentServiceImpl,
    lock::LockServiceImpl,
    repository::RepositoryServiceImpl,
    revision::RevisionServiceImpl,
    storage::StorageServiceImpl,
};
use crate::state::GatewayState;

pub async fn serve(
    addr: SocketAddr,
    state: GatewayState,
    tls_cert: Option<String>,
    tls_key: Option<String>,
) -> Result<(), tonic::transport::Error> {
    let mut builder = Server::builder();

    if let (Some(cert), Some(key)) = (tls_cert, tls_key) {
        let cert_pem = std::fs::read(&cert).expect("failed to read TLS cert");
        let key_pem = std::fs::read(&key).expect("failed to read TLS key");
        let tls = ServerTlsConfig::new().identity(Identity::from_pem(cert_pem, key_pem));
        builder = builder.tls_config(tls)?;
        info!(%addr, "gRPC gateway listening (TLS)");
    } else {
        info!(%addr, "gRPC gateway listening (plaintext)");
    }

    builder
        .add_service(EnvironmentServiceServer::new(EnvironmentServiceImpl {
            state: state.clone(),
        }))
        .add_service(UrcAuthApiServer::new(AuthApiImpl {
            state: state.clone(),
        }))
        .add_service(RepositoryServiceServer::new(RepositoryServiceImpl {
            state: state.clone(),
        }))
        .add_service(RevisionServiceServer::new(RevisionServiceImpl {
            state: state.clone(),
        }))
        .add_service(StorageServiceServer::new(StorageServiceImpl {
            state: state.clone(),
        }))
        .add_service(LockServiceServer::new(LockServiceImpl {
            state: state.clone(),
        }))
        .serve(addr)
        .await
}
