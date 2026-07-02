use tonic::{Request, Response, Status};
use tracing::debug;
use uuid::Uuid;

use crate::proto::rebac::{
    rebac_api_server::RebacApi, CreateResourceRequest, CreateResourceResponse,
    DeleteResourceRequest, DeleteResourceResponse,
};
use crate::services::auth::{extract_claims, resolve_repo_permission};
use crate::state::GatewayState;

pub struct RebacApiImpl {
    pub state: GatewayState,
}

fn parse_resource_id(resource_id: &str) -> Result<Uuid, Status> {
    let hex = resource_id.strip_prefix("urc-").unwrap_or(resource_id);
    Uuid::parse_str(hex).map_err(|_| Status::invalid_argument("invalid resource_id"))
}

#[tonic::async_trait]
impl RebacApi for RebacApiImpl {
    async fn create_resource(
        &self,
        request: Request<CreateResourceRequest>,
    ) -> Result<Response<CreateResourceResponse>, Status> {
        // lore-server creates repositories directly and calls back here to
        // register the resource with LoreVault's permission model; the caller's
        // own bearer token identifies who is doing the creating (and therefore
        // who becomes the resource's owner/admin).
        let claims = extract_claims(request.metadata(), &self.state.jwt)?;
        let req = request.into_inner();
        let repo_id = parse_resource_id(&req.resource_id)?;

        debug!(
            resource_id = %req.resource_id,
            resource_name = %req.resource_name,
            user_id = %claims.sub,
            "rebac: create_resource"
        );

        let mut tx = self
            .state
            .db
            .begin()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        sqlx::query(
            r#"INSERT INTO repositories (id, owner_type, owner_id, name, visibility, default_branch)
               VALUES (?, 'user', ?, ?, 'private', 'main')"#,
        )
        .bind(repo_id.to_string())
        .bind(claims.sub.to_string())
        .bind(&req.resource_name)
        .execute(&mut *tx)
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

        sqlx::query("INSERT INTO repo_permissions (repo_id, user_id, role) VALUES (?, ?, 'admin')")
            .bind(repo_id.to_string())
            .bind(claims.sub.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        tx.commit().await.map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(CreateResourceResponse {}))
    }

    async fn delete_resource(
        &self,
        request: Request<DeleteResourceRequest>,
    ) -> Result<Response<DeleteResourceResponse>, Status> {
        let claims = extract_claims(request.metadata(), &self.state.jwt)?;
        let req = request.into_inner();
        let repo_id = parse_resource_id(&req.resource_id)?;

        debug!(resource_id = %req.resource_id, user_id = %claims.sub, "rebac: delete_resource");

        let permissions = resolve_repo_permission(&self.state, &repo_id, &claims.sub).await?;
        if !permissions.iter().any(|p| p == "admin") {
            return Err(Status::permission_denied("admin role required"));
        }

        // repo_permissions rows cascade on delete via the repositories FK.
        sqlx::query("DELETE FROM repositories WHERE id = ?")
            .bind(repo_id.to_string())
            .execute(&self.state.db)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(DeleteResourceResponse {}))
    }
}
