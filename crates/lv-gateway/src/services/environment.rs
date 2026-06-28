use tonic::{Request, Response, Status};
use tracing::instrument;

use crate::proto::lore_environment_v1::{
    self, environment_service_server::EnvironmentService as EnvironmentServiceV1,
};
use crate::proto::urc::model::{self as urc_model, EnvironmentGetRequest, EnvironmentGetResponse};
use crate::proto::urc::rpc::environment_service_server::EnvironmentService as LegacyEnvironmentService;
use crate::state::GatewayState;

#[derive(Clone)]
pub struct EnvironmentServiceImpl {
    auth_url: String,
}

impl EnvironmentServiceImpl {
    pub fn new(state: &GatewayState) -> Self {
        let auth_url = state
            .server_url
            .replacen("grpcs://", "ucs-auth://", 1);
        Self { auth_url }
    }
}

#[tonic::async_trait]
impl LegacyEnvironmentService for EnvironmentServiceImpl {
    #[instrument(name = "EnvironmentService::Get", skip_all)]
    async fn get(
        &self,
        _request: Request<EnvironmentGetRequest>,
    ) -> Result<Response<EnvironmentGetResponse>, Status> {
        Ok(Response::new(EnvironmentGetResponse {
            environment: Some(urc_model::Environment {
                endpoint: Some(urc_model::EnvironmentEndpoint {
                    auth_url: self.auth_url.clone(),
                    ..Default::default()
                }),
                config: None,
            }),
        }))
    }
}

#[tonic::async_trait]
impl EnvironmentServiceV1 for EnvironmentServiceImpl {
    #[instrument(name = "EnvironmentService::EnvironmentGet", skip_all)]
    async fn environment_get(
        &self,
        _request: Request<lore_environment_v1::EnvironmentGetRequest>,
    ) -> Result<Response<lore_environment_v1::EnvironmentGetResponse>, Status> {
        Ok(Response::new(lore_environment_v1::EnvironmentGetResponse {
            environment: Some(lore_environment_v1::Environment {
                endpoint: Some(lore_environment_v1::Endpoint {
                    auth_url: self.auth_url.clone(),
                    ..Default::default()
                }),
                config: None,
            }),
        }))
    }
}
