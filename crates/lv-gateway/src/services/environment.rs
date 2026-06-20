use tonic::{Request, Response, Status};

use crate::proto::environment::{
    environment_service_server::EnvironmentService, CompressionMode, Config, Endpoint, Environment,
    EnvironmentGetRequest, EnvironmentGetResponse,
};
use crate::state::GatewayState;

pub struct EnvironmentServiceImpl {
    pub state: GatewayState,
}

#[tonic::async_trait]
impl EnvironmentService for EnvironmentServiceImpl {
    async fn environment_get(
        &self,
        _request: Request<EnvironmentGetRequest>,
    ) -> Result<Response<EnvironmentGetResponse>, Status> {
        let url = self.state.server_url.clone();
        Ok(Response::new(EnvironmentGetResponse {
            environment: Some(Environment {
                endpoint: Some(Endpoint {
                    auth_url: url.clone(),
                    repository_url: url.clone(),
                    storage_url: url.clone(),
                    revision_url: url.clone(),
                    lock_url: url.clone(),
                    notification_url: String::new(),
                }),
                config: Some(Config {
                    max_query_batch: 1000,
                    compression_mode: Some(CompressionMode::NoCompression as i32),
                }),
            }),
        }))
    }
}
