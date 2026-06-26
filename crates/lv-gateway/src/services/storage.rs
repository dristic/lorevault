use std::pin::Pin;

use bytes::Bytes;
use deadpool_redis::redis::{AsyncCommands, Script};
use futures::{Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use crate::proto::model::{Address, Fragment, FragmentState};
use crate::proto::storage::{
    storage_service_server::StorageService, CopyRequest, CopyResponse, GetResponse,
    HealResult, MutableCompareAndSwapRequest, MutableCompareAndSwapResponse, MutableLoadRequest,
    MutableLoadResponse, MutableStoreRequest, MutableStoreResponse, PutRequest, PutResponse,
    QueryRequest, QueryResponse, VerifyRequest, VerifyResponse,
};
use crate::state::GatewayState;

// Namespaced Redis key for mutable storage entries.
fn mutable_key(key: &[u8], key_type: u32) -> String {
    format!("mutable:{}:{}", key_type, hex::encode(key))
}

// Atomically compares and optionally swaps a Redis key.
// Returns the value that was present before the operation (empty vec if absent).
// The swap only occurs when the current value equals `expected`.
// Setting `new_value` to an empty slice deletes the key.
static CAS_SCRIPT: &str = r"
local current = redis.call('GET', KEYS[1])
if current == false then current = '' end
if current == ARGV[1] then
    if ARGV[2] == '' then
        redis.call('DEL', KEYS[1])
    else
        redis.call('SET', KEYS[1], ARGV[2])
    end
end
return current
";

pub struct StorageServiceImpl {
    pub state: GatewayState,
}

fn address_to_key(addr: &Address) -> String {
    let context_hex = hex::encode(&addr.context);
    let hash_hex = hex::encode(&addr.hash);
    format!("{context_hex}/{hash_hex}")
}

fn bytes_to_uuid(b: &[u8]) -> Option<Uuid> {
    Uuid::from_slice(b).ok()
}

async fn fetch_one_chunk(
    state: &GatewayState,
    addr: Address,
) -> Result<GetResponse, Status> {
    let key = address_to_key(&addr);
    let data = state
        .blob
        .get(&key)
        .await
        .map_err(|e| Status::not_found(e.to_string()))?;

    let size = data.len() as u64;
    Ok(GetResponse {
        address: Some(addr),
        fragment: Some(Fragment {
            flags: 0,
            size_payload: size as u32,
            size_content: size,
        }),
        payload: data.into(),
    })
}

async fn store_one_chunk(state: &GatewayState, req: PutRequest) -> Result<PutResponse, Status> {
    let addr = req
        .address
        .ok_or_else(|| Status::invalid_argument("address is required"))?;
    let key = address_to_key(&addr);

    if let Some(payload) = req.payload {
        let repo_id = bytes_to_uuid(&addr.context);
        let hash_hex = hex::encode(&addr.hash);
        let size = payload.len() as i64;

        state
            .blob
            .put(&key, Bytes::from(payload.clone()))
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Index chunk in DB if we can determine the repo
        if let Some(repo_id) = repo_id {
            let _ = sqlx::query(
                r#"INSERT INTO chunks (id, repo_id, hash, size_bytes, storage_key)
                   VALUES ($1, $2, $3, $4, $5)
                   ON CONFLICT (repo_id, hash) DO NOTHING"#,
            )
            .bind(Uuid::new_v4())
            .bind(repo_id)
            .bind(&hash_hex)
            .bind(size)
            .bind(&key)
            .execute(&state.db)
            .await;
        }
    }

    Ok(PutResponse {
        address: Some(addr),
    })
}

#[tonic::async_trait]
impl StorageService for StorageServiceImpl {
    type GetStream =
        Pin<Box<dyn Stream<Item = Result<GetResponse, Status>> + Send + 'static>>;
    type GetMetadataStream =
        Pin<Box<dyn Stream<Item = Result<GetResponse, Status>> + Send + 'static>>;
    type PutStream =
        Pin<Box<dyn Stream<Item = Result<PutResponse, Status>> + Send + 'static>>;
    type CopyStream =
        Pin<Box<dyn Stream<Item = Result<CopyResponse, Status>> + Send + 'static>>;

    async fn get(
        &self,
        request: Request<Streaming<Address>>,
    ) -> Result<Response<Self::GetStream>, Status> {
        let state = self.state.clone();
        let mut in_stream = request.into_inner();
        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            while let Some(result) = in_stream.next().await {
                match result {
                    Ok(addr) => {
                        let item = fetch_one_chunk(&state, addr).await;
                        if tx.send(item).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e)).await;
                        break;
                    }
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    async fn get_metadata(
        &self,
        request: Request<Streaming<Address>>,
    ) -> Result<Response<Self::GetMetadataStream>, Status> {
        // Return fragment headers without payload
        let state = self.state.clone();
        let mut in_stream = request.into_inner();
        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            while let Some(result) = in_stream.next().await {
                match result {
                    Ok(addr) => {
                        let key = address_to_key(&addr);
                        let item = match state.blob.exists(&key).await {
                            Ok(true) => Ok(GetResponse {
                                address: Some(addr),
                                fragment: Some(Fragment {
                                    flags: 0,
                                    size_payload: 0,
                                    size_content: 0,
                                }),
                                payload: vec![],
                            }),
                            Ok(false) => Err(Status::not_found("chunk not found")),
                            Err(e) => Err(Status::internal(e.to_string())),
                        };
                        if tx.send(item).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e)).await;
                        break;
                    }
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    async fn put(
        &self,
        request: Request<Streaming<PutRequest>>,
    ) -> Result<Response<Self::PutStream>, Status> {
        let state = self.state.clone();
        let mut in_stream = request.into_inner();
        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            while let Some(result) = in_stream.next().await {
                match result {
                    Ok(put_req) => {
                        let item = store_one_chunk(&state, put_req).await;
                        if tx.send(item).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e)).await;
                        break;
                    }
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    async fn query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        let req = request.into_inner();
        let mut results = Vec::with_capacity(req.addresses.len());

        for addr in &req.addresses {
            let key = address_to_key(addr);
            let state = match self.state.blob.exists(&key).await {
                Ok(true) => FragmentState::FoundInContext as i32,
                Ok(false) => FragmentState::NotFound as i32,
                Err(_) => FragmentState::Unknown as i32,
            };
            results.push(state);
        }

        Ok(Response::new(QueryResponse { results }))
    }

    async fn verify(
        &self,
        request: Request<VerifyRequest>,
    ) -> Result<Response<VerifyResponse>, Status> {
        let req = request.into_inner();
        let addr = req
            .address
            .ok_or_else(|| Status::invalid_argument("address is required"))?;
        let key = address_to_key(&addr);

        let exists = self
            .state
            .blob
            .exists(&key)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(VerifyResponse {
            healed: HealResult::NotAttempted as i32,
            corrupted: !exists,
        }))
    }

    async fn copy(
        &self,
        request: Request<Streaming<CopyRequest>>,
    ) -> Result<Response<Self::CopyStream>, Status> {
        let state = self.state.clone();
        let mut in_stream = request.into_inner();
        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            while let Some(result) = in_stream.next().await {
                match result {
                    Ok(req) => {
                        let item = async {
                            let src_addr = req
                                .source_address
                                .ok_or_else(|| Status::invalid_argument("source_address required"))?;
                            let src_key = address_to_key(&src_addr);
                            let dst_addr = Address {
                                hash: src_addr.hash.clone(),
                                context: req.target_context.clone(),
                            };
                            let dst_key = address_to_key(&dst_addr);

                            let data = state
                                .blob
                                .get(&src_key)
                                .await
                                .map_err(|e| Status::not_found(e.to_string()))?;
                            state
                                .blob
                                .put(&dst_key, data)
                                .await
                                .map_err(|e| Status::internal(e.to_string()))?;

                            Ok(CopyResponse {
                                source_repository_id: req.source_repository_id,
                                source_address: Some(src_addr),
                            })
                        }
                        .await;
                        if tx.send(item).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e)).await;
                        break;
                    }
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    async fn mutable_load(
        &self,
        request: Request<MutableLoadRequest>,
    ) -> Result<Response<MutableLoadResponse>, Status> {
        let req = request.into_inner();
        let redis_key = mutable_key(&req.key, req.key_type);

        let mut conn = self
            .state
            .cache
            .get()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let value: Option<Vec<u8>> = conn
            .get(&redis_key)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(MutableLoadResponse {
            value: value.unwrap_or_default(),
        }))
    }

    async fn mutable_store(
        &self,
        request: Request<MutableStoreRequest>,
    ) -> Result<Response<MutableStoreResponse>, Status> {
        let req = request.into_inner();
        let redis_key = mutable_key(&req.key, req.key_type);

        let mut conn = self
            .state
            .cache
            .get()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        if req.value.is_empty() {
            conn.del::<_, ()>(&redis_key)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;
        } else {
            conn.set::<_, _, ()>(&redis_key, req.value.as_slice())
                .await
                .map_err(|e| Status::internal(e.to_string()))?;
        }

        Ok(Response::new(MutableStoreResponse {}))
    }

    async fn mutable_compare_and_swap(
        &self,
        request: Request<MutableCompareAndSwapRequest>,
    ) -> Result<Response<MutableCompareAndSwapResponse>, Status> {
        let req = request.into_inner();
        let redis_key = mutable_key(&req.key, req.key_type);

        let mut conn = self
            .state
            .cache
            .get()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let current_value: Vec<u8> = Script::new(CAS_SCRIPT)
            .key(&redis_key)
            .arg(req.expected.as_slice())
            .arg(req.value.as_slice())
            .invoke_async(&mut conn)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(MutableCompareAndSwapResponse { current_value }))
    }
}
