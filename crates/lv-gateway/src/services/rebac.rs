use lv_core::models::{RepoRole, Visibility};
use tonic::{Request, Response, Status};
use tracing::debug;
use uuid::Uuid;

use crate::error::map_storage_err;
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

        let mut tx = self.state.storage.begin().await.map_err(map_storage_err)?;

        tx.insert_repository(repo_id, claims.sub, &req.resource_name, Visibility::Private)
            .await
            .map_err(map_storage_err)?;

        // The caller who creates a repository becomes its admin.
        tx.insert_repo_permission(repo_id, claims.sub, RepoRole::Admin)
            .await
            .map_err(map_storage_err)?;

        tx.commit().await.map_err(map_storage_err)?;

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

        let permissions = resolve_repo_permission(&self.state, repo_id, claims.sub).await?;
        if !permissions.iter().any(|p| p == "admin") {
            return Err(Status::permission_denied("admin role required"));
        }

        // repo_permissions rows cascade on delete via the repositories FK.
        self.state
            .storage
            .delete_repository(repo_id)
            .await
            .map_err(map_storage_err)?;

        Ok(Response::new(DeleteResourceResponse {}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_state;

    async fn authed_request<T>(state: &GatewayState, user_id: Uuid, body: T) -> Request<T> {
        let token = lv_auth::jwt::encode(&state.jwt, user_id, "alice").unwrap();
        let mut request = Request::new(body);
        request
            .metadata_mut()
            .insert("authorization", format!("Bearer {token}").parse().unwrap());
        request
    }

    #[tokio::test]
    async fn create_resource_registers_repo_and_makes_caller_admin() {
        let state = test_state().await;
        let user_id = Uuid::new_v4();
        let mut tx = state.storage.begin().await.unwrap();
        tx.insert_user(user_id, "alice", "alice@example.com")
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let repo_id = Uuid::new_v4();
        let request = authed_request(
            &state,
            user_id,
            CreateResourceRequest {
                resource_id: format!("urc-{}", repo_id.as_simple()),
                resource_name: "vault".into(),
            },
        )
        .await;

        let impl_ = RebacApiImpl {
            state: state.clone(),
        };
        impl_.create_resource(request).await.unwrap();

        let repo = state
            .storage
            .get_repository_by_owner_and_name("alice", "vault")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(repo.id, repo_id);
        assert_eq!(
            state
                .storage
                .get_repo_permission(repo_id, user_id)
                .await
                .unwrap(),
            Some(RepoRole::Admin)
        );
    }

    #[tokio::test]
    async fn delete_resource_requires_admin_role() {
        let state = test_state().await;
        let owner_id = Uuid::new_v4();
        let other_id = Uuid::new_v4();
        let repo_id = Uuid::new_v4();

        let mut tx = state.storage.begin().await.unwrap();
        tx.insert_user(owner_id, "alice", "alice@example.com")
            .await
            .unwrap();
        tx.insert_user(other_id, "bob", "bob@example.com")
            .await
            .unwrap();
        tx.insert_repository(repo_id, owner_id, "vault", Visibility::Private)
            .await
            .unwrap();
        tx.insert_repo_permission(repo_id, owner_id, RepoRole::Admin)
            .await
            .unwrap();
        tx.insert_repo_permission(repo_id, other_id, RepoRole::Write)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let impl_ = RebacApiImpl {
            state: state.clone(),
        };

        // A non-admin (write) collaborator cannot delete the repo.
        let request = authed_request(
            &state,
            other_id,
            DeleteResourceRequest {
                resource_id: format!("urc-{}", repo_id.as_simple()),
            },
        )
        .await;
        let err = impl_.delete_resource(request).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::PermissionDenied);
        assert!(state
            .storage
            .get_repository_by_owner_and_name("alice", "vault")
            .await
            .unwrap()
            .is_some());

        // The admin can.
        let request = authed_request(
            &state,
            owner_id,
            DeleteResourceRequest {
                resource_id: format!("urc-{}", repo_id.as_simple()),
            },
        )
        .await;
        impl_.delete_resource(request).await.unwrap();
        assert!(state
            .storage
            .get_repository_by_owner_and_name("alice", "vault")
            .await
            .unwrap()
            .is_none());
    }
}
