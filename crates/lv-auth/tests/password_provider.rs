use std::str::FromStr;
use std::sync::Arc;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use lv_auth::error::AuthError;
use lv_auth::provider::{AuthProvider, NewUser};
use lv_auth::providers::password::PasswordProvider;
use lv_storage::Storage;
use lv_storage_sqlite::SqliteStorage;

/// A fresh in-memory database with migrations applied. `max_connections(1)`
/// keeps every acquired connection pointed at the same in-memory database —
/// SQLite otherwise hands out a brand new empty database per connection.
async fn test_provider() -> PasswordProvider {
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
    PasswordProvider::new(storage)
}

#[tokio::test]
async fn register_then_authenticate_succeeds() {
    let provider = test_provider().await;

    let user_id = provider
        .register_with_password("alice", "alice@example.com", "correct-horse-battery")
        .await
        .unwrap();

    let authed_id = provider
        .authenticate_with_password("alice", "correct-horse-battery")
        .await
        .unwrap();

    assert_eq!(user_id, authed_id);
}

#[tokio::test]
async fn authenticate_by_email_also_works() {
    let provider = test_provider().await;
    provider
        .register_with_password("alice", "alice@example.com", "correct-horse-battery")
        .await
        .unwrap();

    let authed_id = provider
        .authenticate_with_password("alice@example.com", "correct-horse-battery")
        .await
        .unwrap();

    assert!(!authed_id.is_nil());
}

#[tokio::test]
async fn authenticate_rejects_wrong_password() {
    let provider = test_provider().await;
    provider
        .register_with_password("alice", "alice@example.com", "correct-horse-battery")
        .await
        .unwrap();

    let result = provider
        .authenticate_with_password("alice", "wrong-password")
        .await;

    assert!(matches!(result, Err(AuthError::InvalidCredentials)));
}

#[tokio::test]
async fn authenticate_rejects_unknown_user() {
    let provider = test_provider().await;

    let result = provider
        .authenticate_with_password("nobody", "whatever123")
        .await;

    assert!(matches!(result, Err(AuthError::InvalidCredentials)));
}

#[tokio::test]
async fn duplicate_username_registration_conflicts() {
    let provider = test_provider().await;
    provider
        .register_with_password("alice", "alice@example.com", "correct-horse-battery")
        .await
        .unwrap();

    let result = provider
        .register_with_password("alice", "someone-else@example.com", "another-password")
        .await;

    assert!(matches!(result, Err(AuthError::Conflict(_))));
}

#[tokio::test]
async fn register_rejects_invalid_username() {
    let provider = test_provider().await;

    let result = provider
        .register_with_password("ab", "alice@example.com", "correct-horse-battery")
        .await;

    assert!(matches!(result, Err(AuthError::Validation(_))));
}

#[tokio::test]
async fn change_password_requires_current_password() {
    let provider = test_provider().await;
    let user_id = provider
        .register_with_password("alice", "alice@example.com", "correct-horse-battery")
        .await
        .unwrap();

    let result = provider
        .change_password(user_id, "wrong-current", "new-password-123")
        .await;
    assert!(matches!(result, Err(AuthError::InvalidCredentials)));

    provider
        .change_password(user_id, "correct-horse-battery", "new-password-123")
        .await
        .unwrap();

    let authed_id = provider
        .authenticate_with_password("alice", "new-password-123")
        .await
        .unwrap();
    assert_eq!(authed_id, user_id);
}

#[tokio::test]
async fn reset_password_does_not_require_current_password() {
    let provider = test_provider().await;
    let user_id = provider
        .register_with_password("alice", "alice@example.com", "correct-horse-battery")
        .await
        .unwrap();

    provider
        .reset_password(user_id, "brand-new-password")
        .await
        .unwrap();

    let authed_id = provider
        .authenticate_with_password("alice", "brand-new-password")
        .await
        .unwrap();
    assert_eq!(authed_id, user_id);

    // Old password no longer works.
    let result = provider
        .authenticate_with_password("alice", "correct-horse-battery")
        .await;
    assert!(matches!(result, Err(AuthError::InvalidCredentials)));
}

#[tokio::test]
async fn ensure_default_admin_is_idempotent() {
    let provider = test_provider().await;

    provider
        .ensure_default_admin("admin", "admin@example.com", "lorevault-admin-pw")
        .await
        .unwrap();
    // Second call must not error or create a duplicate account.
    provider
        .ensure_default_admin("admin", "admin@example.com", "lorevault-admin-pw")
        .await
        .unwrap();

    let user_id = provider
        .authenticate_with_password("admin", "lorevault-admin-pw")
        .await
        .unwrap();
    assert!(!user_id.is_nil());
}

#[tokio::test]
async fn generic_provider_register_reads_password_from_json() {
    let provider = test_provider().await;

    let user_id = provider
        .register(
            NewUser {
                username: "carol".into(),
                email: "carol@example.com".into(),
            },
            serde_json::json!({ "password": "correct-horse-battery" }),
        )
        .await
        .unwrap();

    let authed_id = provider
        .authenticate(serde_json::json!({
            "login": "carol",
            "password": "correct-horse-battery",
        }))
        .await
        .unwrap();

    assert_eq!(user_id, authed_id);
}
