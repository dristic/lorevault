use sqlx::Row;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::jwt;
use crate::proto::lock::{
    lock_service_server::LockService, AdminLockRequest, AdminLockResponse, Lock, LockRequest,
    LockResponse, QueryRequest, QueryResponse, Resource, StatusRequest, StatusResponse,
    UnlockRequest, UnlockResponse,
};
use crate::state::GatewayState;

pub struct LockServiceImpl {
    pub state: GatewayState,
}

fn bytes_to_uuid(b: &[u8]) -> Option<Uuid> {
    Uuid::from_slice(b).ok()
}

fn make_lock(resource: Resource, owner: &str) -> Lock {
    Lock {
        resource: Some(resource),
        owner: owner.to_owned(),
        locked_at: Some(prost_types::Timestamp {
            seconds: time::OffsetDateTime::now_utc().unix_timestamp(),
            nanos: 0,
        }),
    }
}

async fn repo_for_branch(db: &sqlx::PgPool, branch_id: Uuid) -> Result<Uuid, Status> {
    let row = sqlx::query("SELECT repo_id FROM branches WHERE id = $1")
        .bind(branch_id)
        .fetch_optional(db)
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .ok_or_else(|| Status::not_found("branch not found"))?;
    row.try_get("repo_id").map_err(|e| Status::internal(e.to_string()))
}

#[tonic::async_trait]
impl LockService for LockServiceImpl {
    async fn lock(
        &self,
        request: Request<LockRequest>,
    ) -> Result<Response<LockResponse>, Status> {
        let claims = jwt::extract_claims(request.metadata(), &self.state.jwt_secret)?;
        let resources = request.into_inner().resources;
        let owner = claims.sub.to_string();
        let mut locks = Vec::new();

        for resource in resources {
            let branch_id = bytes_to_uuid(&resource.branch)
                .ok_or_else(|| Status::invalid_argument("invalid branch UUID"))?;
            let repo_id = repo_for_branch(&self.state.db, branch_id).await?;

            sqlx::query(
                r#"INSERT INTO file_locks (id, repo_id, path, locked_by_user_id, workspace_id)
                   VALUES ($1, $2, $3, $4, $5)"#,
            )
            .bind(Uuid::new_v4())
            .bind(repo_id)
            .bind(&resource.description)
            .bind(claims.sub)
            .bind(branch_id.to_string())
            .execute(&self.state.db)
            .await
            .map_err(|e| {
                if e.to_string().contains("duplicate") || e.to_string().contains("unique") {
                    Status::already_exists(format!(
                        "'{}' is already locked",
                        resource.description
                    ))
                } else {
                    Status::internal(e.to_string())
                }
            })?;

            locks.push(make_lock(resource, &owner));
        }

        Ok(Response::new(LockResponse { locks }))
    }

    async fn query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        jwt::extract_claims(request.metadata(), &self.state.jwt_secret)?;
        let req = request.into_inner();

        let branch_bytes = match req.branch.as_deref() {
            Some(b) if !b.is_empty() => b.to_vec(),
            _ => return Ok(Response::new(QueryResponse { result: vec![] })),
        };

        let branch_id = bytes_to_uuid(&branch_bytes)
            .ok_or_else(|| Status::invalid_argument("invalid branch UUID"))?;
        let repo_id = repo_for_branch(&self.state.db, branch_id).await?;

        let rows = sqlx::query(
            r#"SELECT fl.path, fl.locked_by_user_id, fl.acquired_at
               FROM file_locks fl WHERE fl.repo_id = $1"#,
        )
        .bind(repo_id)
        .fetch_all(&self.state.db)
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

        let result: Vec<Lock> = rows
            .into_iter()
            .filter(|r| {
                let uid: Option<Uuid> = r.try_get("locked_by_user_id").ok();
                let path: String = r.try_get("path").unwrap_or_default();
                req.owner
                    .as_ref()
                    .map(|o| uid.map(|u| u.to_string()).as_deref() == Some(o.as_str()))
                    .unwrap_or(true)
                    && req
                        .description
                        .as_ref()
                        .map(|d| d == &path)
                        .unwrap_or(true)
            })
            .map(|r| {
                let path: String = r.try_get("path").unwrap_or_default();
                let uid: Uuid = r.try_get("locked_by_user_id").unwrap_or(Uuid::nil());
                let acquired: time::OffsetDateTime =
                    r.try_get("acquired_at").unwrap_or_else(|_| time::OffsetDateTime::now_utc());
                Lock {
                    resource: Some(Resource {
                        branch: branch_bytes.clone(),
                        hash: vec![],
                        description: path,
                    }),
                    owner: uid.to_string(),
                    locked_at: Some(prost_types::Timestamp {
                        seconds: acquired.unix_timestamp(),
                        nanos: 0,
                    }),
                }
            })
            .collect();

        Ok(Response::new(QueryResponse { result }))
    }

    async fn status(
        &self,
        request: Request<StatusRequest>,
    ) -> Result<Response<StatusResponse>, Status> {
        jwt::extract_claims(request.metadata(), &self.state.jwt_secret)?;
        let resources = request.into_inner().resources;
        let mut locks = Vec::new();

        for resource in resources {
            let branch_id = match bytes_to_uuid(&resource.branch) {
                Some(id) => id,
                None => continue,
            };
            let repo_id = match repo_for_branch(&self.state.db, branch_id).await {
                Ok(id) => id,
                Err(_) => continue,
            };

            let row = sqlx::query(
                r#"SELECT locked_by_user_id, acquired_at FROM file_locks
                   WHERE repo_id = $1 AND path = $2"#,
            )
            .bind(repo_id)
            .bind(&resource.description)
            .fetch_optional(&self.state.db)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

            if let Some(r) = row {
                let uid: Uuid = r.try_get("locked_by_user_id").unwrap_or(Uuid::nil());
                let acquired: time::OffsetDateTime =
                    r.try_get("acquired_at").unwrap_or_else(|_| time::OffsetDateTime::now_utc());
                locks.push(Lock {
                    resource: Some(resource),
                    owner: uid.to_string(),
                    locked_at: Some(prost_types::Timestamp {
                        seconds: acquired.unix_timestamp(),
                        nanos: 0,
                    }),
                });
            }
        }

        Ok(Response::new(StatusResponse { locks }))
    }

    async fn unlock(
        &self,
        request: Request<UnlockRequest>,
    ) -> Result<Response<UnlockResponse>, Status> {
        let claims = jwt::extract_claims(request.metadata(), &self.state.jwt_secret)?;
        let resources = request.into_inner().resources;
        let mut unlocked = Vec::new();

        for resource in resources {
            let branch_id = match bytes_to_uuid(&resource.branch) {
                Some(id) => id,
                None => continue,
            };
            let repo_id = match repo_for_branch(&self.state.db, branch_id).await {
                Ok(id) => id,
                Err(_) => continue,
            };

            sqlx::query(
                "DELETE FROM file_locks WHERE repo_id = $1 AND path = $2 AND locked_by_user_id = $3",
            )
            .bind(repo_id)
            .bind(&resource.description)
            .bind(claims.sub)
            .execute(&self.state.db)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

            unlocked.push(resource);
        }

        Ok(Response::new(UnlockResponse { resources: unlocked }))
    }

    async fn admin_lock(
        &self,
        _: Request<AdminLockRequest>,
    ) -> Result<Response<AdminLockResponse>, Status> {
        Err(Status::unimplemented("admin lock not supported"))
    }
}
