//! Shared test fixtures for the gateway's gRPC service unit tests.
use std::str::FromStr;
use std::sync::Arc;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use lv_auth::jwt::JwtConfig;
use lv_storage::Storage;
use lv_storage_sqlite::SqliteStorage;

use crate::state::GatewayState;

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

pub(crate) fn test_jwt_config(ttl_seconds: i64) -> JwtConfig {
    JwtConfig::from_rsa_pem(
        "https://lorevault.test".into(),
        vec!["lorevault.test".into()],
        TEST_PRIVATE_KEY_PEM.as_bytes(),
        ttl_seconds,
    )
    .unwrap()
}

/// A `GatewayState` backed by a fresh in-memory database with migrations
/// applied. `max_connections(1)` keeps every acquired connection pointed at
/// the same in-memory database — SQLite otherwise hands out a brand new
/// empty database per connection.
pub(crate) async fn test_state() -> GatewayState {
    let opts = SqliteConnectOptions::from_str("sqlite::memory:").unwrap();
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .unwrap();
    sqlx::query("PRAGMA foreign_keys=ON")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::migrate!("../../migrations").run(&pool).await.unwrap();

    let storage: Arc<dyn Storage> = Arc::new(SqliteStorage::new(pool));
    let jwt = Arc::new(test_jwt_config(3600));

    GatewayState::new(
        storage,
        jwt,
        "https://lorevault.test".into(),
        "https://lorevault.test".into(),
    )
}
