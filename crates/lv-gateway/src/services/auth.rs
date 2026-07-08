use lv_core::models::{AuthSessionState, RepoRole};
use time::{Duration, OffsetDateTime};
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::error::map_storage_err;
use crate::proto::auth_api::{target_user, ResourcePermission};
use crate::proto::auth_api::{
    urc_auth_api_server::UrcAuthApi, CheckUserPermissionRequest, CheckUserPermissionResponse,
    ExchangeApiKeyForUserTokenRequest, ExchangeApiKeyForUserTokenResponse,
    ExchangeExternalTokenForUserTokenRequest, ExchangeExternalTokenForUserTokenResponse,
    ExchangeUserTokenForMultiresourceTokenRequest, ExchangeUserTokenForMultiresourceTokenResponse,
    GetAuthSessionRequest, GetAuthSessionResponse, GetProviderUserIdRequest,
    GetProviderUserIdResponse, GetUserIdRequest, GetUserIdResponse, GetUserInfoRequest,
    GetUserInfoResponse, HealthCheckRequest, HealthCheckResponse, LookupUserPermissionsRequest,
    LookupUserPermissionsResponse, RefreshAuthSessionRequest, RefreshAuthSessionResponse,
    StartAuthSessionRequest, StartAuthSessionResponse, UserToken, VerifyUserRequest,
    VerifyUserResponse,
};
use crate::state::GatewayState;

pub struct AuthApiImpl {
    pub state: GatewayState,
}

fn make_user_token(claims: &lv_auth::jwt::Claims, token_str: String) -> UserToken {
    UserToken {
        user_token: token_str,
        expires_at: claims.exp,
        user_id: claims.sub.to_string(),
        user_name: claims.preferred_username.clone(),
    }
}

pub(crate) fn extract_claims(
    meta: &tonic::metadata::MetadataMap,
    config: &lv_auth::jwt::JwtConfig,
) -> Result<lv_auth::jwt::Claims, Status> {
    let token = meta
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| Status::unauthenticated("missing authorization header"))?;

    lv_auth::jwt::decode(config, token).map_err(|e| match e {
        lv_auth::error::AuthError::TokenExpired => Status::unauthenticated("token expired"),
        _ => Status::unauthenticated("invalid token"),
    })
}

pub(crate) async fn resolve_repo_permission(
    state: &GatewayState,
    repo_id: Uuid,
    user_id: Uuid,
) -> Result<Vec<String>, Status> {
    let role = state
        .storage
        .get_repo_permission(repo_id, user_id)
        .await
        .map_err(map_storage_err)?;

    let permissions: Vec<String> = match role {
        Some(RepoRole::Admin) => vec!["read".into(), "write".into(), "admin".into()],
        Some(RepoRole::Write) => vec!["read".into(), "write".into()],
        Some(RepoRole::Read) => vec!["read".into()],
        None => vec![],
    };

    Ok(permissions)
}

#[tonic::async_trait]
impl UrcAuthApi for AuthApiImpl {
    #[tracing::instrument(name = "AuthApi::health_check", skip_all, level = "debug")]
    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        Ok(Response::new(HealthCheckResponse {
            status: "ok".into(),
        }))
    }

    #[tracing::instrument(
        name = "AuthApi::exchange_api_key_for_user_token",
        skip_all,
        level = "debug"
    )]
    async fn exchange_api_key_for_user_token(
        &self,
        request: Request<ExchangeApiKeyForUserTokenRequest>,
    ) -> Result<Response<ExchangeApiKeyForUserTokenResponse>, Status> {
        let api_key = request.into_inner().api_key;
        let hash = lv_auth::token::hash_api_token(&api_key);

        let user = self
            .state
            .storage
            .find_user_by_token_hash(&hash)
            .await
            .map_err(map_storage_err)?
            .ok_or_else(|| Status::unauthenticated("invalid api key"))?;
        let (user_id, username) = (user.id, user.username);

        let token_str = lv_auth::jwt::encode(&self.state.jwt, user_id, &username)
            .map_err(|e| Status::internal(e.to_string()))?;
        let claims = lv_auth::jwt::decode(&self.state.jwt, &token_str)
            .map_err(|_| Status::internal("failed to decode freshly issued token"))?;

        Ok(Response::new(ExchangeApiKeyForUserTokenResponse {
            user_token: Some(make_user_token(&claims, token_str)),
        }))
    }

    #[tracing::instrument(
        name = "AuthApi::exchange_user_token_for_multiresource_token",
        skip_all,
        level = "debug"
    )]
    async fn exchange_user_token_for_multiresource_token(
        &self,
        request: Request<ExchangeUserTokenForMultiresourceTokenRequest>,
    ) -> Result<Response<ExchangeUserTokenForMultiresourceTokenResponse>, Status> {
        let base_claims = extract_claims(request.metadata(), &self.state.jwt)?;
        let resource_ids = request.into_inner().resource_id;

        // Resource IDs arrive as "urc-{hex}" from the Lore CLI; strip the prefix before parsing.
        let repos: Vec<Uuid> = resource_ids
            .iter()
            .filter_map(|s| {
                let hex = s.strip_prefix("urc-").unwrap_or(s);
                Uuid::parse_str(hex).ok()
            })
            .collect();

        let token_str = lv_auth::jwt::encode_scoped(
            &self.state.jwt,
            base_claims.sub,
            &base_claims.preferred_username,
            repos,
        )
        .map_err(|e| Status::internal(e.to_string()))?;
        let scoped_claims = lv_auth::jwt::decode(&self.state.jwt, &token_str)
            .map_err(|_| Status::internal("failed to decode freshly issued token"))?;

        Ok(Response::new(
            ExchangeUserTokenForMultiresourceTokenResponse {
                token: Some(make_user_token(&scoped_claims, token_str)),
            },
        ))
    }

    #[tracing::instrument(name = "AuthApi::start_auth_session", skip_all, level = "debug")]
    async fn start_auth_session(
        &self,
        _: Request<StartAuthSessionRequest>,
    ) -> Result<Response<StartAuthSessionResponse>, Status> {
        let code_bytes: [u8; 16] = rand::random();
        let session_code = hex::encode(code_bytes);

        let expires_at = OffsetDateTime::now_utc() + Duration::seconds(600);
        self.state
            .storage
            .start_auth_session(&session_code, expires_at)
            .await
            .map_err(map_storage_err)?;

        let login_url = format!("{}/login?session={session_code}", self.state.web_url);

        Ok(Response::new(StartAuthSessionResponse {
            session_code,
            login_url,
        }))
    }

    #[tracing::instrument(name = "AuthApi::get_auth_session", skip_all, level = "debug")]
    async fn get_auth_session(
        &self,
        request: Request<GetAuthSessionRequest>,
    ) -> Result<Response<GetAuthSessionResponse>, Status> {
        let session_code = request.into_inner().session_code;
        let now = OffsetDateTime::now_utc();

        let session = self
            .state
            .storage
            .get_auth_session(&session_code, now)
            .await
            .map_err(map_storage_err)?
            .ok_or_else(|| Status::not_found("session expired or not found"))?;

        let user_token = if session.state == AuthSessionState::Complete {
            let token = session
                .token
                .ok_or_else(|| Status::internal("completed session missing token"))?;
            let username = session
                .username
                .ok_or_else(|| Status::internal("completed session missing username"))?;

            let claims = lv_auth::jwt::decode(&self.state.jwt, &token)
                .map_err(|_| Status::internal("failed to read token expiry"))?;

            // Use username as user_id so the CLI credential store is keyed by
            // the human-readable name and --identity <username> works.
            Some(UserToken {
                user_token: token,
                user_id: username.clone(),
                user_name: username,
                expires_at: claims.exp,
            })
        } else {
            None
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

    #[tracing::instrument(
        name = "AuthApi::exchange_external_token_for_user_token",
        skip_all,
        level = "debug"
    )]
    async fn exchange_external_token_for_user_token(
        &self,
        request: Request<ExchangeExternalTokenForUserTokenRequest>,
    ) -> Result<Response<ExchangeExternalTokenForUserTokenResponse>, Status> {
        let req = request.into_inner();
        match req.token_type.as_str() {
            "api-key" => {
                let hash = lv_auth::token::hash_api_token(&req.external_token);
                let user = self
                    .state
                    .storage
                    .find_user_by_token_hash(&hash)
                    .await
                    .map_err(map_storage_err)?
                    .ok_or_else(|| Status::unauthenticated("invalid api key"))?;
                let (user_id, username) = (user.id, user.username);

                let token_str = lv_auth::jwt::encode(&self.state.jwt, user_id, &username)
                    .map_err(|e| Status::internal(e.to_string()))?;
                let claims = lv_auth::jwt::decode(&self.state.jwt, &token_str)
                    .map_err(|_| Status::internal("failed to decode freshly issued token"))?;

                Ok(Response::new(ExchangeExternalTokenForUserTokenResponse {
                    user_token: Some(make_user_token(&claims, token_str)),
                }))
            }
            other => Err(Status::unimplemented(format!(
                "token type '{other}' is not supported"
            ))),
        }
    }

    #[tracing::instrument(name = "AuthApi::check_user_permission", skip_all, level = "debug")]
    async fn check_user_permission(
        &self,
        request: Request<CheckUserPermissionRequest>,
    ) -> Result<Response<CheckUserPermissionResponse>, Status> {
        let claims = extract_claims(request.metadata(), &self.state.jwt)?;
        let req = request.into_inner();

        let user_id = match req.target_user.and_then(|t| t.user) {
            // Asking for permissions for a different token than the claims.
            Some(target_user::User::UserToken(token)) => {
                lv_auth::jwt::decode(&self.state.jwt, &token)
                    .map_err(|_| Status::unauthenticated("invalid user token"))?
                    .sub
            }
            // Asking for permissions on behalf of this request's user.
            None => claims.sub,
        };

        tracing::debug!(%user_id, "target_user");

        let mut allowed_resource_permission = Vec::new();
        let mut denied_resource_permission = Vec::new();

        for resource_id in req.resource_id {
            let hex = resource_id.strip_prefix("urc-").unwrap_or(&resource_id);
            let permission = match Uuid::parse_str(hex) {
                Ok(repo_id) => resolve_repo_permission(&self.state, repo_id, user_id).await?,
                Err(_) => vec![],
            };

            if permission.is_empty() {
                denied_resource_permission.push(ResourcePermission {
                    resource_id,
                    permission,
                })
            } else {
                allowed_resource_permission.push(ResourcePermission {
                    resource_id,
                    permission,
                })
            }
        }

        let response = CheckUserPermissionResponse {
            allowed_resource_permission,
            denied_resource_permission,
        };

        Ok(Response::new(response))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{test_jwt_config, test_state};
    use tonic::metadata::MetadataMap;

    #[tokio::test]
    async fn resolve_repo_permission_maps_roles_to_permission_lists() {
        let state = test_state().await;
        let owner_id = Uuid::new_v4();
        let repo_id = Uuid::new_v4();

        let mut tx = state.storage.begin().await.unwrap();
        tx.insert_user(owner_id, "alice", "alice@example.com")
            .await
            .unwrap();
        tx.insert_repository(
            repo_id,
            owner_id,
            "vault",
            lv_core::models::Visibility::Private,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        // No permission row yet.
        assert!(resolve_repo_permission(&state, repo_id, owner_id)
            .await
            .unwrap()
            .is_empty());

        state
            .storage
            .set_repo_permission(repo_id, owner_id, RepoRole::Read)
            .await
            .unwrap();
        assert_eq!(
            resolve_repo_permission(&state, repo_id, owner_id)
                .await
                .unwrap(),
            vec!["read"]
        );

        state
            .storage
            .set_repo_permission(repo_id, owner_id, RepoRole::Write)
            .await
            .unwrap();
        assert_eq!(
            resolve_repo_permission(&state, repo_id, owner_id)
                .await
                .unwrap(),
            vec!["read", "write"]
        );

        state
            .storage
            .set_repo_permission(repo_id, owner_id, RepoRole::Admin)
            .await
            .unwrap();
        assert_eq!(
            resolve_repo_permission(&state, repo_id, owner_id)
                .await
                .unwrap(),
            vec!["read", "write", "admin"]
        );
    }

    #[test]
    fn extract_claims_rejects_missing_authorization_header() {
        let config = test_jwt_config(3600);
        let meta = MetadataMap::new();

        let err = extract_claims(&meta, &config).unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }

    #[test]
    fn extract_claims_accepts_valid_bearer_token() {
        let config = test_jwt_config(3600);
        let user_id = Uuid::new_v4();
        let token = lv_auth::jwt::encode(&config, user_id, "alice").unwrap();

        let mut meta = MetadataMap::new();
        meta.insert("authorization", format!("Bearer {token}").parse().unwrap());

        let claims = extract_claims(&meta, &config).unwrap();
        assert_eq!(claims.sub, user_id);
    }

    #[test]
    fn extract_claims_rejects_expired_token() {
        // jsonwebtoken applies a default 60s leeway around `exp`, so the
        // token must be expired by more than that to be rejected.
        let config = test_jwt_config(-120);
        let token = lv_auth::jwt::encode(&config, Uuid::new_v4(), "alice").unwrap();

        let mut meta = MetadataMap::new();
        meta.insert("authorization", format!("Bearer {token}").parse().unwrap());

        let err = extract_claims(&meta, &config).unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
        assert_eq!(err.message(), "token expired");
    }

    #[tokio::test]
    async fn exchange_api_key_for_user_token_rejects_unknown_key() {
        let state = test_state().await;
        let impl_ = AuthApiImpl { state };

        let request = Request::new(ExchangeApiKeyForUserTokenRequest {
            api_key: "lv_does-not-exist".into(),
        });
        let err = impl_
            .exchange_api_key_for_user_token(request)
            .await
            .unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }

    #[tokio::test]
    async fn exchange_api_key_for_user_token_succeeds_for_known_key() {
        let state = test_state().await;
        let user_id = Uuid::new_v4();
        let mut tx = state.storage.begin().await.unwrap();
        tx.insert_user(user_id, "alice", "alice@example.com")
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let hash = lv_auth::token::hash_api_token("lv_test-token");
        state
            .storage
            .insert_api_token(Uuid::new_v4(), user_id, "ci", &hash)
            .await
            .unwrap();

        let impl_ = AuthApiImpl { state };
        let request = Request::new(ExchangeApiKeyForUserTokenRequest {
            api_key: "lv_test-token".into(),
        });
        let response = impl_
            .exchange_api_key_for_user_token(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.user_token.unwrap().user_id, user_id.to_string());
    }
}
