use std::pin::Pin;

use futures::Stream;
use sqlx::Row;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::jwt;
use crate::proto::model::Repository as ProtoRepository;
use crate::proto::repository::{
    repository_get_request::Query, repository_service_server::RepositoryService,
    RepositoryCreateRequest, RepositoryCreateResponse, RepositoryDeleteRequest,
    RepositoryDeleteResponse, RepositoryGetRequest, RepositoryGetResponse,
    RepositoryListRequest, RepositoryListResponse, RepositoryMetadataGetRequest,
    RepositoryMetadataGetResponse, RepositoryMetadataSetRequest, RepositoryMetadataSetResponse,
};
use crate::state::GatewayState;

pub struct RepositoryServiceImpl {
    pub state: GatewayState,
}

fn uuid_to_bytes(id: Uuid) -> Vec<u8> {
    id.as_bytes().to_vec()
}

fn bytes_to_uuid(b: &[u8]) -> Result<Uuid, Status> {
    Uuid::from_slice(b).map_err(|_| Status::invalid_argument("invalid UUID bytes"))
}

fn row_to_proto(row: &sqlx::postgres::PgRow) -> Result<ProtoRepository, Status> {
    let id: Uuid = row.try_get("id").map_err(|e| Status::internal(e.to_string()))?;
    let name: String = row.try_get("name").map_err(|e| Status::internal(e.to_string()))?;
    let description: Option<String> = row.try_get("description").ok();
    let default_branch: String = row
        .try_get("default_branch")
        .map_err(|e| Status::internal(e.to_string()))?;
    let default_branch_uuid: Option<Uuid> = row.try_get("default_branch_uuid").ok().flatten();
    let owner_id: Uuid = row.try_get("owner_id").map_err(|e| Status::internal(e.to_string()))?;
    let created_at: time::OffsetDateTime = row
        .try_get("created_at")
        .map_err(|e| Status::internal(e.to_string()))?;
    let repo_metadata: Option<Vec<u8>> = row.try_get("repo_metadata").ok().flatten();

    Ok(ProtoRepository {
        id: uuid_to_bytes(id),
        name,
        description: description.unwrap_or_default(),
        default_branch_id: default_branch_uuid.map(uuid_to_bytes).unwrap_or_default(),
        default_branch_name: default_branch,
        creator: owner_id.to_string(),
        created: created_at.unix_timestamp() as u64,
        metadata: repo_metadata.unwrap_or_default(),
    })
}

const SELECT_REPO_COLS: &str =
    "id, name, description, default_branch, default_branch_uuid, owner_id, created_at, repo_metadata";

#[tonic::async_trait]
impl RepositoryService for RepositoryServiceImpl {
    type RepositoryListStream =
        Pin<Box<dyn Stream<Item = Result<RepositoryListResponse, Status>> + Send + 'static>>;

    async fn repository_create(
        &self,
        request: Request<RepositoryCreateRequest>,
    ) -> Result<Response<RepositoryCreateResponse>, Status> {
        let claims = jwt::extract_claims(request.metadata(), &self.state.jwt_secret)?;
        let req = request.into_inner();

        let repo_id = bytes_to_uuid(&req.id)?;
        let default_branch_id = if !req.default_branch_id.is_empty() {
            Some(bytes_to_uuid(&req.default_branch_id)?)
        } else {
            None
        };

        let row = sqlx::query(
            r#"INSERT INTO repositories
                   (id, owner_type, owner_id, name, description, default_branch, default_branch_uuid)
               VALUES ($1, 'user', $2, $3, $4, $5, $6)
               RETURNING id, name, description, default_branch, default_branch_uuid,
                         owner_id, created_at, repo_metadata"#,
        )
        .bind(repo_id)
        .bind(claims.sub)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.default_branch_name)
        .bind(default_branch_id)
        .fetch_one(&self.state.db)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate") || e.to_string().contains("unique") {
                Status::already_exists("repository already exists")
            } else {
                Status::internal(e.to_string())
            }
        })?;

        // Create the default branch entry if an ID was provided
        if let Some(branch_id) = default_branch_id {
            sqlx::query(
                r#"INSERT INTO branches (id, repo_id, name, head_revision_hash)
                   VALUES ($1, $2, $3, '')
                   ON CONFLICT (id) DO NOTHING"#,
            )
            .bind(branch_id)
            .bind(repo_id)
            .bind(&req.default_branch_name)
            .execute(&self.state.db)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        }

        let repo = row_to_proto(&row)?;
        Ok(Response::new(RepositoryCreateResponse {
            repository: Some(repo),
        }))
    }

    async fn repository_delete(
        &self,
        request: Request<RepositoryDeleteRequest>,
    ) -> Result<Response<RepositoryDeleteResponse>, Status> {
        let claims = jwt::extract_claims(request.metadata(), &self.state.jwt_secret)?;
        let repo_id = bytes_to_uuid(&request.into_inner().id)?;

        let row = sqlx::query(&format!(
            "DELETE FROM repositories WHERE id = $1 AND owner_id = $2 RETURNING {SELECT_REPO_COLS}"
        ))
        .bind(repo_id)
        .bind(claims.sub)
        .fetch_optional(&self.state.db)
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .ok_or(Status::not_found("repository not found"))?;

        let repo = row_to_proto(&row)?;
        Ok(Response::new(RepositoryDeleteResponse {
            repository: Some(repo),
        }))
    }

    async fn repository_get(
        &self,
        request: Request<RepositoryGetRequest>,
    ) -> Result<Response<RepositoryGetResponse>, Status> {
        let claims = jwt::extract_claims(request.metadata(), &self.state.jwt_secret)?;
        let req = request.into_inner();

        let row = match req.query {
            Some(Query::Id(id_bytes)) => {
                let id = bytes_to_uuid(&id_bytes)?;
                sqlx::query(&format!(
                    "SELECT {SELECT_REPO_COLS} FROM repositories WHERE id = $1"
                ))
                .bind(id)
                .fetch_optional(&self.state.db)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
            }
            Some(Query::Name(name)) => {
                sqlx::query(&format!(
                    "SELECT {SELECT_REPO_COLS} FROM repositories WHERE name = $1 AND owner_id = $2"
                ))
                .bind(&name)
                .bind(claims.sub)
                .fetch_optional(&self.state.db)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
            }
            None => return Err(Status::invalid_argument("query is required")),
        }
        .ok_or(Status::not_found("repository not found"))?;

        let repo = row_to_proto(&row)?;
        Ok(Response::new(RepositoryGetResponse {
            repository: Some(repo),
        }))
    }

    async fn repository_list(
        &self,
        request: Request<RepositoryListRequest>,
    ) -> Result<Response<Self::RepositoryListStream>, Status> {
        let claims = jwt::extract_claims(request.metadata(), &self.state.jwt_secret)?;

        let rows = sqlx::query(&format!(
            "SELECT {SELECT_REPO_COLS} FROM repositories WHERE owner_id = $1 ORDER BY created_at DESC"
        ))
        .bind(claims.sub)
        .fetch_all(&self.state.db)
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

        let items: Vec<Result<RepositoryListResponse, Status>> = rows
            .iter()
            .map(|row| {
                row_to_proto(row).map(|repo| RepositoryListResponse {
                    repository: Some(repo),
                })
            })
            .collect();

        Ok(Response::new(Box::pin(futures::stream::iter(items))))
    }

    async fn repository_metadata_get(
        &self,
        request: Request<RepositoryMetadataGetRequest>,
    ) -> Result<Response<RepositoryMetadataGetResponse>, Status> {
        jwt::extract_claims(request.metadata(), &self.state.jwt_secret)?;
        let id = bytes_to_uuid(&request.into_inner().id)?;

        let row = sqlx::query("SELECT repo_metadata FROM repositories WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.state.db)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or(Status::not_found("repository not found"))?;

        let metadata: Option<Vec<u8>> = row.try_get("repo_metadata").ok().flatten();
        Ok(Response::new(RepositoryMetadataGetResponse {
            metadata: metadata.unwrap_or_default(),
        }))
    }

    async fn repository_metadata_set(
        &self,
        request: Request<RepositoryMetadataSetRequest>,
    ) -> Result<Response<RepositoryMetadataSetResponse>, Status> {
        let claims = jwt::extract_claims(request.metadata(), &self.state.jwt_secret)?;
        let req = request.into_inner();
        let id = bytes_to_uuid(&req.id)?;

        let result = sqlx::query(
            r#"UPDATE repositories
               SET repo_metadata = $1
               WHERE id = $2 AND owner_id = $3
                 AND (repo_metadata = $4 OR (repo_metadata IS NULL AND octet_length($4) = 0))
               RETURNING repo_metadata"#,
        )
        .bind(&req.updated)
        .bind(id)
        .bind(claims.sub)
        .bind(&req.expected)
        .fetch_optional(&self.state.db)
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .ok_or(Status::aborted("metadata conflict or repository not found"))?;

        let metadata: Option<Vec<u8>> = result.try_get("repo_metadata").ok().flatten();
        Ok(Response::new(RepositoryMetadataSetResponse {
            metadata: metadata.unwrap_or_default(),
        }))
    }
}
