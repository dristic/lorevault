use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rsa::{pkcs8::DecodePrivateKey, traits::PublicKeyParts, RsaPrivateKey};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{AuthError, Result};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoreResourcePermission {
    resource_id: String,
    permission: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub iss: String,
    pub sub: Uuid,
    pub exp: i64,
    pub iat: i64,
    /// Audience — hostnames this token is valid for (used by Lore CLI domain check).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub aud: Vec<String>,
    /// Display name (Lore CLI reads `name` from the JWT).
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub preferred_username: String,
    /// Required by lore-server's JWTUserInfo deserializer; identifies the auth environment.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub env: String,
    /// Resources that this token has access to.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub resources: Vec<LoreResourcePermission>,
    /// Identity provider — required by lore-server's AuthorizationToken deserializer.
    /// Always emitted (even as empty string) so decode::<AuthorizationToken> succeeds
    /// and the resources field is read rather than falling back to JWTUserInfo.
    pub idp: String,
}

pub struct JwtConfig {
    pub issuer: String,
    /// Hostnames this server serves; written into `aud` so Lore's domain check passes.
    pub audience: Vec<String>,
    pub encoding_key: EncodingKey,
    pub decoding_key: DecodingKey,
    /// TTL in seconds
    pub ttl_seconds: i64,
    /// Precomputed JWKS
    pub jwks: serde_json::Value,
}

impl JwtConfig {
    pub fn from_rsa_pem(
        issuer: String,
        audience: Vec<String>,
        private_pem: &[u8],
        ttl_seconds: i64,
    ) -> Result<Self> {
        let pem_str =
            std::str::from_utf8(private_pem).map_err(|e| AuthError::Internal(e.to_string()))?;

        let rsa_key = RsaPrivateKey::from_pkcs8_pem(pem_str)
            .map_err(|e| AuthError::Internal(e.to_string()))?;

        let encoding_key = EncodingKey::from_rsa_pem(private_pem)
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        let decoding_key = DecodingKey::from_rsa_raw_components(
            &rsa_key.n().to_bytes_be(),
            &rsa_key.e().to_bytes_be(),
        );

        let n = URL_SAFE_NO_PAD.encode(rsa_key.n().to_bytes_be());
        let e = URL_SAFE_NO_PAD.encode(rsa_key.e().to_bytes_be());

        let jwks = serde_json::json!({
            "keys": [{ "kty": "RSA", "use": "sig", "alg": "RS256", "kid": "1", "n": n, "e": e }]
        });

        Ok(Self {
            issuer,
            audience,
            encoding_key,
            decoding_key,
            ttl_seconds,
            jwks,
        })
    }
}

pub fn encode(
    config: &JwtConfig,
    user_id: Uuid,
    username: &str,
) -> Result<String> {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let claims = Claims {
        iss: config.issuer.clone(),
        aud: config.audience.clone(),
        sub: user_id,
        name: username.to_string(),
        preferred_username: username.to_string(),
        env: config.issuer.clone(),
        iat: now,
        exp: now + config.ttl_seconds,
        resources: vec![],
        idp: String::new(),
    };

    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("1".to_string());
    jsonwebtoken::encode(&header, &claims, &config.encoding_key)
        .map_err(|e| AuthError::Internal(e.to_string()))
}

/// Encode a multiresource token scoped to specific repository IDs.
pub fn encode_scoped(
    config: &JwtConfig,
    user_id: Uuid,
    username: &str,
    repos: Vec<Uuid>,
) -> Result<String> {
    let now = OffsetDateTime::now_utc().unix_timestamp();

    let resources = repos
        .iter()
        .map(|r| LoreResourcePermission {
            resource_id: format!("urc-{}", r.as_simple()),
            permission: vec!["read".to_string(), "write".to_string()],
        })
        .collect::<Vec<LoreResourcePermission>>();

    let claims = Claims {
        iss: config.issuer.clone(),
        aud: config.audience.clone(),
        sub: user_id,
        name: username.to_string(),
        preferred_username: username.to_string(),
        env: config.issuer.clone(),
        iat: now,
        exp: now + config.ttl_seconds,
        resources,
        idp: String::new(),
    };

    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("1".to_string());
    jsonwebtoken::encode(&header, &claims, &config.encoding_key)
        .map_err(|e| AuthError::Internal(e.to_string()))
}

pub fn decode(config: &JwtConfig, token: &str) -> Result<Claims> {
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[&config.issuer]);
    validation.validate_aud = false;
    jsonwebtoken::decode::<Claims>(token, &config.decoding_key, &validation)
        .map(|d| d.claims)
        .map_err(|e| {
            tracing::warn!(error = %e, "JWT decode failed");
            match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
                _ => AuthError::TokenInvalid,
            }
        })
}
