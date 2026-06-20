use std::net::SocketAddr;

use tracing::info;

/// Starts the gRPC gateway server.
/// Service registration is stubbed until proto stubs are generated via build.rs.
pub async fn serve(addr: SocketAddr) -> Result<(), tonic::transport::Error> {
    info!(%addr, "gRPC gateway listening (no services registered yet)");
    // Pending implementation: register RepoService, CasService, LockService, AdminService
    // then call tonic::transport::Server::builder().add_service(...).serve(addr).await
    std::future::pending::<Result<(), tonic::transport::Error>>().await
}
