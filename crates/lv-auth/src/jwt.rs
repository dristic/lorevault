use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{AuthError, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject: user ID
    pub sub: Uuid,
    /// Expiry (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
}

pub struct JwtConfig {
    pub secret: String,
    /// TTL in seconds
    pub ttl_secs: i64,
}

impl JwtConfig {
    pub fn encode(&self, user_id: Uuid) -> Result<String> {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let claims = Claims {
            sub: user_id,
            iat: now,
            exp: now + self.ttl_secs,
        };
        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| AuthError::Internal(e.to_string()))
    }

    pub fn decode(&self, token: &str) -> Result<Claims> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::new(Algorithm::HS256),
        )
        .map(|d| d.claims)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
            _ => AuthError::TokenInvalid,
        })
    }
}
