use tonic::{Request, Status};
use uuid::Uuid;

/// gRPC metadata key carrying the API token.
pub const AUTH_HEADER: &str = "authorization";

/// Parsed auth context injected into every gRPC call after the interceptor runs.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Uuid,
}

/// Extracts the Bearer token from gRPC metadata.
pub fn extract_bearer(req: &Request<()>) -> Result<&str, Status> {
    let meta = req
        .metadata()
        .get(AUTH_HEADER)
        .ok_or_else(|| Status::unauthenticated("missing authorization header"))?;

    let value = meta
        .to_str()
        .map_err(|_| Status::unauthenticated("invalid authorization header"))?;

    value
        .strip_prefix("Bearer ")
        .ok_or_else(|| Status::unauthenticated("authorization must be Bearer token"))
}
