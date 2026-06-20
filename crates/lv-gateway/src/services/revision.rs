use std::pin::Pin;

use futures::Stream;
use sqlx::Row;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::jwt;
use crate::proto::model::{Branch as ProtoBranch, RevisionItem};
use crate::proto::revision::{
    branch_get_request::Query, revision_list_request::Start,
    revision_service_server::RevisionService, BranchCreateRequest, BranchCreateResponse,
    BranchDeleteRequest, BranchDeleteResponse, BranchGetRequest, BranchGetResponse,
    BranchListRequest, BranchListResponse, BranchMetadataGetRequest, BranchMetadataGetResponse,
    BranchMetadataSetRequest, BranchMetadataSetResponse, BranchPushRequest, BranchPushResponse,
    RevisionListRequest, RevisionListResponse,
};
use crate::state::GatewayState;

pub struct RevisionServiceImpl {
    pub state: GatewayState,
}

fn uuid_to_bytes(id: Uuid) -> Vec<u8> {
    id.as_bytes().to_vec()
}

fn bytes_to_uuid(b: &[u8]) -> Result<Uuid, Status> {
    Uuid::from_slice(b).map_err(|_| Status::invalid_argument("invalid UUID bytes"))
}

fn hex_to_bytes(s: &str) -> Vec<u8> {
    hex::decode(s).unwrap_or_default()
}

fn row_to_branch(row: &sqlx::postgres::PgRow) -> Result<ProtoBranch, Status> {
    let id: Uuid = row.try_get("id").map_err(|e| Status::internal(e.to_string()))?;
    let name: String = row.try_get("name").map_err(|e| Status::internal(e.to_string()))?;
    let head: String = row
        .try_get("head_revision_hash")
        .map_err(|e| Status::internal(e.to_string()))?;
    let creator: String = row.try_get("creator").unwrap_or_default();
    let category: String = row.try_get("category").unwrap_or_default();
    let created_at: time::OffsetDateTime = row
        .try_get("created_at")
        .map_err(|e| Status::internal(e.to_string()))?;
    let deleted: bool = row.try_get("deleted").unwrap_or(false);
    let metadata: Option<Vec<u8>> = row.try_get("metadata").ok().flatten();

    Ok(ProtoBranch {
        id: uuid_to_bytes(id),
        name,
        creator,
        category,
        created: created_at.unix_timestamp() as u64,
        latest: hex_to_bytes(&head),
        deleted,
        metadata: metadata.unwrap_or_default(),
        stack: vec![],
    })
}

const SELECT_BRANCH_COLS: &str =
    "id, name, head_revision_hash, creator, category, created_at, deleted, metadata";

#[tonic::async_trait]
impl RevisionService for RevisionServiceImpl {
    type BranchListStream =
        Pin<Box<dyn Stream<Item = Result<BranchListResponse, Status>> + Send + 'static>>;

    async fn branch_create(
        &self,
        request: Request<BranchCreateRequest>,
    ) -> Result<Response<BranchCreateResponse>, Status> {
        let claims = jwt::extract_claims(request.metadata(), &self.state.jwt_secret)?;
        let repo_id = jwt::scoped_repo(&claims)?;
        let req = request.into_inner();

        let branch_id = bytes_to_uuid(&req.id)?;
        let creator = req
            .creator
            .clone()
            .unwrap_or_else(|| claims.sub.to_string());
        let parent_hex = req
            .stack
            .first()
            .map(|bp| hex::encode(&bp.revision_signature))
            .unwrap_or_default();

        sqlx::query(
            r#"INSERT INTO branches (id, repo_id, name, head_revision_hash, creator, category)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
        )
        .bind(branch_id)
        .bind(repo_id)
        .bind(&req.name)
        .bind(&parent_hex)
        .bind(&creator)
        .bind(&req.category)
        .execute(&self.state.db)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate") || e.to_string().contains("unique") {
                Status::already_exists("branch already exists")
            } else {
                Status::internal(e.to_string())
            }
        })?;

        let row = sqlx::query(&format!(
            "SELECT {SELECT_BRANCH_COLS} FROM branches WHERE id = $1"
        ))
        .bind(branch_id)
        .fetch_one(&self.state.db)
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

        let branch = row_to_branch(&row)?;
        Ok(Response::new(BranchCreateResponse {
            branch: Some(branch),
        }))
    }

    async fn branch_delete(
        &self,
        request: Request<BranchDeleteRequest>,
    ) -> Result<Response<BranchDeleteResponse>, Status> {
        let claims = jwt::extract_claims(request.metadata(), &self.state.jwt_secret)?;
        let repo_id = jwt::scoped_repo(&claims)?;
        let branch_id = bytes_to_uuid(&request.into_inner().id)?;

        let row = sqlx::query(&format!(
            "UPDATE branches SET deleted = true WHERE id = $1 AND repo_id = $2 RETURNING {SELECT_BRANCH_COLS}"
        ))
        .bind(branch_id)
        .bind(repo_id)
        .fetch_optional(&self.state.db)
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .ok_or(Status::not_found("branch not found"))?;

        let branch = row_to_branch(&row)?;
        Ok(Response::new(BranchDeleteResponse {
            branch: Some(branch),
        }))
    }

    async fn branch_get(
        &self,
        request: Request<BranchGetRequest>,
    ) -> Result<Response<BranchGetResponse>, Status> {
        let claims = jwt::extract_claims(request.metadata(), &self.state.jwt_secret)?;
        let repo_id = jwt::scoped_repo(&claims)?;
        let req = request.into_inner();

        let row = match req.query {
            Some(Query::Id(id_bytes)) => {
                let id = bytes_to_uuid(&id_bytes)?;
                sqlx::query(&format!(
                    "SELECT {SELECT_BRANCH_COLS} FROM branches WHERE id = $1 AND repo_id = $2"
                ))
                .bind(id)
                .bind(repo_id)
                .fetch_optional(&self.state.db)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
            }
            Some(Query::Name(name)) => {
                sqlx::query(&format!(
                    "SELECT {SELECT_BRANCH_COLS} FROM branches WHERE name = $1 AND repo_id = $2"
                ))
                .bind(&name)
                .bind(repo_id)
                .fetch_optional(&self.state.db)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
            }
            None => return Err(Status::invalid_argument("query is required")),
        }
        .ok_or(Status::not_found("branch not found"))?;

        let branch = row_to_branch(&row)?;
        Ok(Response::new(BranchGetResponse {
            branch: Some(branch),
        }))
    }

    async fn branch_list(
        &self,
        request: Request<BranchListRequest>,
    ) -> Result<Response<Self::BranchListStream>, Status> {
        let claims = jwt::extract_claims(request.metadata(), &self.state.jwt_secret)?;
        let repo_id = jwt::scoped_repo(&claims)?;
        let include_deleted = request.into_inner().include_deleted;

        let rows = if include_deleted {
            sqlx::query(&format!(
                "SELECT {SELECT_BRANCH_COLS} FROM branches WHERE repo_id = $1 ORDER BY created_at"
            ))
            .bind(repo_id)
            .fetch_all(&self.state.db)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
        } else {
            sqlx::query(&format!(
                "SELECT {SELECT_BRANCH_COLS} FROM branches WHERE repo_id = $1 AND deleted = false ORDER BY created_at"
            ))
            .bind(repo_id)
            .fetch_all(&self.state.db)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
        };

        let items: Vec<Result<BranchListResponse, Status>> = rows
            .iter()
            .map(|row| {
                row_to_branch(row).map(|branch| BranchListResponse {
                    branch: Some(branch),
                })
            })
            .collect();

        Ok(Response::new(Box::pin(futures::stream::iter(items))))
    }

    async fn branch_push(
        &self,
        request: Request<BranchPushRequest>,
    ) -> Result<Response<BranchPushResponse>, Status> {
        let claims = jwt::extract_claims(request.metadata(), &self.state.jwt_secret)?;
        let repo_id = jwt::scoped_repo(&claims)?;
        let req = request.into_inner();

        let branch_id = bytes_to_uuid(&req.id)?;
        let sig_hex = hex::encode(&req.revision_signature);

        // Verify branch belongs to scoped repo
        let exists: bool = sqlx::query(
            "SELECT id FROM branches WHERE id = $1 AND repo_id = $2",
        )
        .bind(branch_id)
        .bind(repo_id)
        .fetch_optional(&self.state.db)
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .is_some();

        if !exists {
            return Err(Status::not_found("branch not found"));
        }

        // Assign next revision number
        let count: i64 = sqlx::query(
            "SELECT COUNT(*) AS cnt FROM revisions WHERE repo_id = $1",
        )
        .bind(repo_id)
        .fetch_one(&self.state.db)
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .try_get::<i64, _>("cnt")
        .unwrap_or(0);
        let revision_number = (count + 1) as u64;

        // Insert revision (idempotent)
        sqlx::query(
            r#"INSERT INTO revisions (id, repo_id, hash, author_id, number)
               VALUES ($1, $2, $3, $4, $5)
               ON CONFLICT (repo_id, hash) DO NOTHING"#,
        )
        .bind(Uuid::new_v4())
        .bind(repo_id)
        .bind(&sig_hex)
        .bind(claims.sub)
        .bind(revision_number as i64)
        .execute(&self.state.db)
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

        // Advance branch head
        sqlx::query(
            "UPDATE branches SET head_revision_hash = $1, updated_at = now() WHERE id = $2",
        )
        .bind(&sig_hex)
        .bind(branch_id)
        .execute(&self.state.db)
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(BranchPushResponse {
            revision_signature: req.revision_signature,
            revision_number,
            fast_forward_merged: req.fast_forward_merge,
            message: None,
        }))
    }

    async fn branch_metadata_get(
        &self,
        request: Request<BranchMetadataGetRequest>,
    ) -> Result<Response<BranchMetadataGetResponse>, Status> {
        let claims = jwt::extract_claims(request.metadata(), &self.state.jwt_secret)?;
        let repo_id = jwt::scoped_repo(&claims)?;
        let id = bytes_to_uuid(&request.into_inner().id)?;

        let row = sqlx::query(
            "SELECT metadata FROM branches WHERE id = $1 AND repo_id = $2",
        )
        .bind(id)
        .bind(repo_id)
        .fetch_optional(&self.state.db)
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .ok_or(Status::not_found("branch not found"))?;

        let metadata: Option<Vec<u8>> = row.try_get("metadata").ok().flatten();
        Ok(Response::new(BranchMetadataGetResponse {
            metadata: metadata.unwrap_or_default(),
        }))
    }

    async fn branch_metadata_set(
        &self,
        request: Request<BranchMetadataSetRequest>,
    ) -> Result<Response<BranchMetadataSetResponse>, Status> {
        let claims = jwt::extract_claims(request.metadata(), &self.state.jwt_secret)?;
        let repo_id = jwt::scoped_repo(&claims)?;
        let req = request.into_inner();
        let id = bytes_to_uuid(&req.id)?;

        let result = sqlx::query(
            r#"UPDATE branches
               SET metadata = $1
               WHERE id = $2 AND repo_id = $3
                 AND (metadata = $4 OR (metadata IS NULL AND octet_length($4) = 0))
               RETURNING metadata"#,
        )
        .bind(&req.updated)
        .bind(id)
        .bind(repo_id)
        .bind(&req.expected)
        .fetch_optional(&self.state.db)
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .ok_or(Status::aborted("metadata conflict or branch not found"))?;

        let metadata: Option<Vec<u8>> = result.try_get("metadata").ok().flatten();
        Ok(Response::new(BranchMetadataSetResponse {
            metadata: metadata.unwrap_or_default(),
        }))
    }

    async fn revision_list(
        &self,
        request: Request<RevisionListRequest>,
    ) -> Result<Response<RevisionListResponse>, Status> {
        let claims = jwt::extract_claims(request.metadata(), &self.state.jwt_secret)?;
        let repo_id = jwt::scoped_repo(&claims)?;
        let req = request.into_inner();

        // Find the starting signature if an identifier was given
        if let Some(Start::Identifier(id_msg)) = &req.start {
            let branch_id = bytes_to_uuid(&id_msg.branch_id)?;
            // Validate branch belongs to repo
            let _ = sqlx::query(
                "SELECT id FROM branches WHERE id = $1 AND repo_id = $2",
            )
            .bind(branch_id)
            .bind(repo_id)
            .fetch_optional(&self.state.db)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        }

        // Return up to 100 revisions for the repo, newest first
        let rows = sqlx::query(
            "SELECT hash, number FROM revisions WHERE repo_id = $1 ORDER BY number DESC NULLS LAST LIMIT 100",
        )
        .bind(repo_id)
        .fetch_all(&self.state.db)
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

        let items: Vec<RevisionItem> = rows
            .iter()
            .map(|r| {
                let hash: String = r.try_get("hash").unwrap_or_default();
                let number: Option<i64> = r.try_get("number").ok().flatten();
                RevisionItem {
                    number: number.unwrap_or(0) as u64,
                    signature: hex::decode(&hash).unwrap_or_default(),
                    metadata: vec![],
                    state: vec![],
                }
            })
            .collect();

        let oldest_sig = items.last().map(|i| i.signature.clone());
        let newest_sig = items.first().map(|i| i.signature.clone());

        Ok(Response::new(RevisionListResponse {
            items,
            signature_forward: newest_sig,
            signature_backward: oldest_sig,
        }))
    }
}
