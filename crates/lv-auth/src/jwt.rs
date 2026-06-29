use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rsa::{pkcs8::DecodePrivateKey, traits::PublicKeyParts, RsaPrivateKey};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{AuthError, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Issuer
    pub iss: String,
    /// Subject: user ID
    pub sub: Uuid,
    /// Expiry (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Organization scope — None for personal (unscoped) tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org: Option<Uuid>,
}

pub struct JwtConfig {
    pub issuer: String,
    pub encoding_key: EncodingKey,
    pub decoding_key: DecodingKey,
    /// TTL in seconds
    pub ttl_seconds: i64,
    /// Precomputed JWKS
    pub jwks: serde_json::Value,
}

impl JwtConfig {
    pub fn from_rsa_pem(issuer: String, private_pem: &[u8], ttl_seconds: i64) -> Result<Self> {
        let encoding_key = EncodingKey::from_rsa_pem(private_pem)
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        let decoding_key = DecodingKey::from_rsa_pem(private_pem)
            .map_err(|e| AuthError::Internal(e.to_string()))?;

        // Pregenerate our JWKS
        let pem_str =
            std::str::from_utf8(private_pem).map_err(|e| AuthError::Internal(e.to_string()))?;

        let rsa_key = RsaPrivateKey::from_pkcs8_pem(pem_str)
            .map_err(|e| AuthError::Internal(e.to_string()))?;

        let n = URL_SAFE_NO_PAD.encode(rsa_key.n().to_bytes_be());
        let e = URL_SAFE_NO_PAD.encode(rsa_key.e().to_bytes_be());

        let jwks = serde_json::json!({
            "keys": [{ "kty": "RSA", "use": "sig", "alg": "RS256", "kid": "1", "n": n, "e": e }]
        });

        Ok(Self {
            issuer,
            encoding_key,
            decoding_key,
            ttl_seconds,
            jwks,
        })
    }
}

pub fn encode(config: &JwtConfig, user_id: Uuid, org_id: Option<Uuid>) -> Result<String> {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let claims = Claims {
        iss: config.issuer.clone(),
        sub: user_id,
        iat: now,
        exp: now + config.ttl_seconds,
        org: org_id,
    };

    jsonwebtoken::encode(
        &Header::new(Algorithm::RS256),
        &claims,
        &config.encoding_key,
    )
    .map_err(|e| AuthError::Internal(e.to_string()))
}

pub fn decode(config: &JwtConfig, token: &str) -> Result<Claims> {
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[&config.issuer]);
    jsonwebtoken::decode::<Claims>(token, &config.decoding_key, &validation)
        .map(|d| d.claims)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
            _ => AuthError::TokenInvalid,
        })
}
