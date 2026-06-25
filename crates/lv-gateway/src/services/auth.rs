use deadpool_redis::redis::AsyncCommands;
use sqlx::Row;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::jwt::{self, GatewayClaims};
use crate::proto::auth_api::{
    urc_auth_api_server::UrcAuthApi, CheckUserPermissionRequest, CheckUserPermissionResponse,
    ExchangeApiKeyForUserTokenRequest, ExchangeApiKeyForUserTokenResponse,
    ExchangeExternalTokenForUserTokenRequest, ExchangeExternalTokenForUserTokenResponse,
    ExchangeUserTokenForMultiresourceTokenRequest,
    ExchangeUserTokenForMultiresourceTokenResponse, GetAuthSessionRequest,
    GetAuthSessionResponse, GetProviderUserIdRequest, GetProviderUserIdResponse,
    GetUserIdRequest, GetUserIdResponse, GetUserInfoRequest, GetUserInfoResponse,
    HealthCheckRequest, HealthCheckResponse, LookupUserPermissionsRequest,
    LookupUserPermissionsResponse, RefreshAuthSessionRequest, RefreshAuthSessionResponse,
    StartAuthSessionRequest, StartAuthSessionResponse, UserToken, VerifyUserRequest,
    VerifyUserResponse,
};
use crate::state::GatewayState;

pub struct AuthApiImpl {
    pub state: GatewayState,
}

fn make_user_token(claims: &GatewayClaims, token_str: String, username: &str) -> UserToken {
    UserToken {
        user_token: token_str,
        expires_at: claims.exp,
        user_id: claims.sub.to_string(),
        user_name: username.to_owned(),
    }
}

#[tonic::async_trait]
impl UrcAuthApi for AuthApiImpl {
    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        Ok(Response::new(HealthCheckResponse {
            status: "ok".into(),
        }))
    }

    async fn exchange_api_key_for_user_token(
        &self,
        request: Request<ExchangeApiKeyForUserTokenRequest>,
    ) -> Result<Response<ExchangeApiKeyForUserTokenResponse>, Status> {
        let api_key = request.into_inner().api_key;
        let hash = lv_auth::token::hash_api_token(&api_key);

        let row = sqlx::query(
            r#"SELECT t.user_id, u.username
               FROM api_tokens t
               JOIN users u ON u.id = t.user_id
               WHERE t.token_hash = $1"#,
        )
        .bind(&hash)
        .fetch_optional(&self.state.db)
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .ok_or_else(|| Status::unauthenticated("invalid api key"))?;

        let user_id: Uuid = row
            .try_get("user_id")
            .map_err(|e| Status::internal(e.to_string()))?;
        let username: String = row
            .try_get("username")
            .map_err(|e| Status::internal(e.to_string()))?;

        let claims = jwt::new_claims(user_id, &username, &self.state.issuer, vec![], self.state.jwt_ttl_secs);
        let token_str = jwt::encode_claims(&claims, &self.state.jwt_secret)
            .map_err(Status::internal)?;

        Ok(Response::new(ExchangeApiKeyForUserTokenResponse {
            user_token: Some(make_user_token(&claims, token_str, &username)),
        }))
    }

    async fn exchange_user_token_for_multiresource_token(
        &self,
        request: Request<ExchangeUserTokenForMultiresourceTokenRequest>,
    ) -> Result<Response<ExchangeUserTokenForMultiresourceTokenResponse>, Status> {
        let base_claims = jwt::extract_claims(request.metadata(), &self.state.jwt_secret)?;
        let resource_ids = request.into_inner().resource_id;

        let repos: Vec<Uuid> = resource_ids
            .iter()
            .filter_map(|s| Uuid::parse_str(s).ok())
            .collect();

        let scoped = jwt::new_claims(base_claims.sub, &base_claims.name, &self.state.issuer, repos, self.state.jwt_ttl_secs);
        let token_str = jwt::encode_claims(&scoped, &self.state.jwt_secret)
            .map_err(Status::internal)?;

        let username: String = sqlx::query("SELECT username FROM users WHERE id = $1")
            .bind(base_claims.sub)
            .fetch_optional(&self.state.db)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .and_then(|r| r.try_get::<String, _>("username").ok())
            .unwrap_or_default();

        Ok(Response::new(
            ExchangeUserTokenForMultiresourceTokenResponse {
                token: Some(make_user_token(&scoped, token_str, &username)),
            },
        ))
    }

    async fn start_auth_session(
        &self,
        _: Request<StartAuthSessionRequest>,
    ) -> Result<Response<StartAuthSessionResponse>, Status> {
        let code_bytes: [u8; 16] = rand::random();
        let session_code = hex::encode(code_bytes);

        let mut conn = self
            .state
            .cache
            .get()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let key = format!("auth:session:{session_code}");
        conn.set_ex::<_, _, ()>(&key, r#"{"state":"pending"}"#, 600u64)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let login_url = format!("{}/login?session={session_code}", self.state.web_url);

        Ok(Response::new(StartAuthSessionResponse {
            session_code,
            login_url,
        }))
    }

    async fn get_auth_session(
        &self,
        request: Request<GetAuthSessionRequest>,
    ) -> Result<Response<GetAuthSessionResponse>, Status> {
        let session_code = request.into_inner().session_code;

        let mut conn = self
            .state
            .cache
            .get()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let key = format!("auth:session:{session_code}");
        let raw: Option<String> = conn
            .get(&key)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let raw = raw.ok_or_else(|| Status::not_found("session expired or not found"))?;

        let session: serde_json::Value =
            serde_json::from_str(&raw).map_err(|e| Status::internal(e.to_string()))?;

        let user_token = match session["state"].as_str() {
            Some("complete") => Some(UserToken {
                user_token: session["token"].as_str().unwrap_or_default().to_string(),
                user_id: session["user_id"].as_str().unwrap_or_default().to_string(),
                user_name: session["username"].as_str().unwrap_or_default().to_string(),
                expires_at: session["expires_at"].as_i64().unwrap_or_default(),
            }),
            _ => None,
        };

        Ok(Response::new(GetAuthSessionResponse { user_token }))
    }

    // ── Stub implementations ──────────────────────────────────────────────────

    async fn refresh_auth_session(
        &self,
        _: Request<RefreshAuthSessionRequest>,
    ) -> Result<Response<RefreshAuthSessionResponse>, Status> {
        Err(Status::unimplemented("not supported"))
    }

    async fn verify_user(
        &self,
        _: Request<VerifyUserRequest>,
    ) -> Result<Response<VerifyUserResponse>, Status> {
        Err(Status::unimplemented("not supported"))
    }

    async fn exchange_external_token_for_user_token(
        &self,
        request: Request<ExchangeExternalTokenForUserTokenRequest>,
    ) -> Result<Response<ExchangeExternalTokenForUserTokenResponse>, Status> {
        let req = request.into_inner();
        match req.token_type.as_str() {
            "api-key" => {
                let hash = lv_auth::token::hash_api_token(&req.external_token);
                let row = sqlx::query(
                    r#"SELECT t.user_id, u.username
                       FROM api_tokens t
                       JOIN users u ON u.id = t.user_id
                       WHERE t.token_hash = $1"#,
                )
                .bind(&hash)
                .fetch_optional(&self.state.db)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
                .ok_or_else(|| Status::unauthenticated("invalid api key"))?;

                let user_id: Uuid = row.try_get("user_id").map_err(|e| Status::internal(e.to_string()))?;
                let username: String = row.try_get("username").map_err(|e| Status::internal(e.to_string()))?;

                let claims = jwt::new_claims(user_id, &username, &self.state.issuer, vec![], self.state.jwt_ttl_secs);
                let token_str = jwt::encode_claims(&claims, &self.state.jwt_secret)
                    .map_err(Status::internal)?;

                Ok(Response::new(ExchangeExternalTokenForUserTokenResponse {
                    user_token: Some(make_user_token(&claims, token_str, &username)),
                }))
            }
            other => Err(Status::unimplemented(format!("token type '{other}' is not supported"))),
        }
    }

    async fn check_user_permission(
        &self,
        _: Request<CheckUserPermissionRequest>,
    ) -> Result<Response<CheckUserPermissionResponse>, Status> {
        Err(Status::unimplemented("not supported"))
    }

    async fn lookup_user_permissions(
        &self,
        _: Request<LookupUserPermissionsRequest>,
    ) -> Result<Response<LookupUserPermissionsResponse>, Status> {
        Err(Status::unimplemented("not supported"))
    }

    async fn get_user_info(
        &self,
        _: Request<GetUserInfoRequest>,
    ) -> Result<Response<GetUserInfoResponse>, Status> {
        Err(Status::unimplemented("not supported"))
    }

    async fn get_user_id(
        &self,
        _: Request<GetUserIdRequest>,
    ) -> Result<Response<GetUserIdResponse>, Status> {
        Err(Status::unimplemented("not supported"))
    }

    async fn get_provider_user_id(
        &self,
        _: Request<GetProviderUserIdRequest>,
    ) -> Result<Response<GetProviderUserIdResponse>, Status> {
        Err(Status::unimplemented("not supported"))
    }
}
