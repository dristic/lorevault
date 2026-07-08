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

pub fn encode(config: &JwtConfig, user_id: Uuid, username: &str) -> Result<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    // Test-only fixture key — never used outside the test binary.
    const TEST_PRIVATE_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC0pTr0Js+PZbmN
apNhLY/sLZlxQo4SXkVw+F64TE+uws3lRM9NOOcPIH5zkkgyZOlN/708RzMHEggT
Z6qrYQarjkZTCaIN9WYlpfDR7l3dQp2OJBw+PoYU/CQdBK01hsOtUwS7ocX/awdP
rkYdx+OpqlbkHQH16VzQJlMo4EnEtxo07uLU2FmNeJEKtfmL32kg1mQvvP3XT/6R
LpAxyuHQ2Xf+nxY2HsPdms3pU3OnIuKP5YhNFuurrFAjZfX05z+HCNPHiDIEEbVo
ulroj/tQjryxB/8g8xqA8YbpepHtXJcYFqvurSrKXPWQjfQHi9jJN4xlRsR1kWsE
34AmdTqVAgMBAAECggEADyWkqBjCAjDiKmazlWwr236mVV4iig1ADt0wmg0CCHIa
sB0BMeUx0K2llLzBE4KtImZtgGqq726WYUQpxiWEWOm84VUXMsrvJfyAUSYG1ljx
25uRB7IX7ZYH1CwSdwDGExg5Nx91OfnIOujuxav/XbhkAUwiYDORXf28rtqBrP4W
DOty7b/6Wq8AVyNs4qW0BlfwxYbZ7FLj6h+yo5oTDRlIDRaiVbvJsJaYhl3pAdBW
BAl6CzGDKkqSPTk43gKGUuzEk+N8ZoVIIsZZp4AfpqkX1ZF5R9D6tFWiN/E5vp0V
m2tUCWld1SpOIw/9nhBSUFh1MaxX2TafLBJSqJADMQKBgQD2dxwSVYRwj1xB3O/I
rmtpHbuB7G1eLY1Tk5IaKMcnsBb8oMQ3wTLZNPAzPeLJv0f4ca0bqQ6kuizp3RMR
kNRjJLQOsqF1+gMypegz9WUCm1pMHCHXRnOsXhxLTwnXOJ3A/SezpZ5jjI1GLT41
Rz1caUkRnOmbENTAc8OlBZmcZQKBgQC7okShXd84QyVJnfCtCStvUJccYU52HLgj
3kPymmAQfWE9K00E5WWgFLhjZcM4Kv4PSaZ3uGBCwoFF7jVyHfedsZqi+T/uwldX
GJAA91j9udEtdK6t3fTQRA5bJl/tLW8DXO/Peg8I5wn8EvrtcG02/T0gRbB3sJRx
4ubgmIpKcQKBgGiKQRftug1cYY92PSbsBJdDi0Mim4k03Rs0HuaFoWPOJxHkxxW3
FvBWqgOyHj3gqpBQ91IiNRnd9isEIJB01AFxkgYh8qZt82lKQeG4Fq4yYuyhiiEb
uvjDulCfJ9doJlGzj2F9wF8NQOchTZ+fpgFKjzmvSs8BJpyy/atDYtKZAoGAHA97
ZgqM3HQmOmk1WhtZ9I6/2o2u1zkaTLrrvHdb0Ht/tE8qeIX5+cO/g5XvaRH85rpj
+9mGA9Xk0Vl7grJ6mom6D49pAULtHuhceNiE5YUJhFvD19quxwq2fukxRV4bEQyw
DH47i2BJ/Pm1rxa2LpgWsSHa7ztoJ9QAJSyK2fECgYEA48jVkXCbL8YB2RVCa4pn
NJEAG0gHin2mXm5jguZDiTiKkR3oWpcvI+5tqf1hPOz/0zSTsircJzy+yn1fV6cP
555euvrYUa9d9xiHHYJCk5zon0l0CraTxiEyOnimda5a7aFkPPm91uxaLDPvHbHa
u8tgI2zyRqyWT4Jf9qqqFVw=
-----END PRIVATE KEY-----
";

    fn test_config(ttl_seconds: i64) -> JwtConfig {
        JwtConfig::from_rsa_pem(
            "https://lorevault.test".into(),
            vec!["lorevault.test".into()],
            TEST_PRIVATE_KEY_PEM.as_bytes(),
            ttl_seconds,
        )
        .unwrap()
    }

    #[test]
    fn encode_then_decode_roundtrips_claims() {
        let config = test_config(3600);
        let user_id = Uuid::new_v4();
        let token = encode(&config, user_id, "alice").unwrap();

        let claims = decode(&config, &token).unwrap();
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.preferred_username, "alice");
        assert_eq!(claims.iss, "https://lorevault.test");
        assert!(claims.resources.is_empty());
    }

    #[test]
    fn encode_scoped_includes_repo_permissions() {
        let config = test_config(3600);
        let repo_id = Uuid::new_v4();
        let token = encode_scoped(&config, Uuid::new_v4(), "bob", vec![repo_id]).unwrap();

        let claims = decode(&config, &token).unwrap();
        assert_eq!(claims.resources.len(), 1);
        assert_eq!(
            claims.resources[0].resource_id,
            format!("urc-{}", repo_id.as_simple())
        );
        assert_eq!(claims.resources[0].permission, vec!["read", "write"]);
    }

    #[test]
    fn decode_rejects_expired_token() {
        // jsonwebtoken applies a default 60s leeway around `exp`, so the
        // token must be expired by more than that to be rejected.
        let config = test_config(-120);
        let token = encode(&config, Uuid::new_v4(), "alice").unwrap();

        assert!(matches!(
            decode(&config, &token),
            Err(AuthError::TokenExpired)
        ));
    }

    #[test]
    fn decode_rejects_token_from_different_issuer() {
        let issuer_a = test_config(3600);
        let mut issuer_b = test_config(3600);
        issuer_b.issuer = "https://other.test".into();

        let token = encode(&issuer_a, Uuid::new_v4(), "alice").unwrap();
        assert!(matches!(
            decode(&issuer_b, &token),
            Err(AuthError::TokenInvalid)
        ));
    }

    #[test]
    fn decode_rejects_tampered_token() {
        let config = test_config(3600);
        let mut token = encode(&config, Uuid::new_v4(), "alice").unwrap();
        token.push('x');

        assert!(decode(&config, &token).is_err());
    }
}
