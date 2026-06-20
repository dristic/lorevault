use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tonic::Status;
use uuid::Uuid;

/// JWT claims used by the gRPC gateway.  The `repos` field carries the
/// resource-scoped repository UUIDs produced by `ExchangeUserTokenForMultiresourceToken`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GatewayClaims {
    pub sub: Uuid,
    pub exp: i64,
    pub iat: i64,
    #[serde(default)]
    pub repos: Vec<Uuid>,
}

pub fn new_claims(user_id: Uuid, repos: Vec<Uuid>, ttl_secs: i64) -> GatewayClaims {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    GatewayClaims {
        sub: user_id,
        exp: now + ttl_secs,
        iat: now,
        repos,
    }
}

pub fn encode_claims(claims: &GatewayClaims, secret: &str) -> Result<String, String> {
    encode(
        &Header::new(Algorithm::HS256),
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| e.to_string())
}

pub fn decode_claims(token: &str, secret: &str) -> Result<GatewayClaims, Status> {
    decode::<GatewayClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map(|d| d.claims)
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
            Status::unauthenticated("token expired")
        }
        _ => Status::unauthenticated("invalid token"),
    })
}

/// Extract and decode a Bearer JWT from gRPC request metadata.
pub fn extract_claims(
    meta: &tonic::metadata::MetadataMap,
    secret: &str,
) -> Result<GatewayClaims, Status> {
    let token = meta
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| Status::unauthenticated("missing authorization header"))?;

    decode_claims(token, secret)
}

/// Return the first (and typically only) scoped repository UUID from claims.
pub fn scoped_repo(claims: &GatewayClaims) -> Result<Uuid, Status> {
    claims
        .repos
        .first()
        .copied()
        .ok_or_else(|| Status::unauthenticated("token has no repository scope"))
}
