use tonic::{Request, Response, Status};
use tracing::debug;

use crate::proto::rebac::{
    rebac_api_server::RebacApi, CreateResourceRequest, CreateResourceResponse,
    DeleteResourceRequest, DeleteResourceResponse,
};

pub struct RebacApiImpl;

#[tonic::async_trait]
impl RebacApi for RebacApiImpl {
    async fn create_resource(
        &self,
        request: Request<CreateResourceRequest>,
    ) -> Result<Response<CreateResourceResponse>, Status> {
        let req = request.into_inner();
        debug!(resource_id = %req.resource_id, resource_name = %req.resource_name, "rebac: create_resource");
        Ok(Response::new(CreateResourceResponse {}))
    }

    async fn delete_resource(
        &self,
        request: Request<DeleteResourceRequest>,
    ) -> Result<Response<DeleteResourceResponse>, Status> {
        let req = request.into_inner();
        debug!(resource_id = %req.resource_id, "rebac: delete_resource");
        Ok(Response::new(DeleteResourceResponse {}))
    }
}
