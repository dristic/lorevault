use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use lv_core::models::{RepoRole, Visibility};
use lv_storage::{Storage, StorageError};
use lv_storage_sqlite::SqliteStorage;

/// A fresh in-memory database with migrations applied. `max_connections(1)`
/// keeps every acquired connection pointed at the same in-memory database —
/// SQLite otherwise hands out a brand new empty database per connection.
async fn test_storage() -> SqliteStorage {
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

    SqliteStorage::new(pool)
}

async fn insert_user(storage: &SqliteStorage, username: &str, email: &str) -> Uuid {
    let user_id = Uuid::new_v4();
    let mut tx = storage.begin().await.unwrap();
    tx.insert_user(user_id, username, email).await.unwrap();
    tx.commit().await.unwrap();
    user_id
}

async fn insert_user_with_password(storage: &SqliteStorage, username: &str, email: &str) -> Uuid {
    let user_id = Uuid::new_v4();
    let mut tx = storage.begin().await.unwrap();
    tx.insert_user(user_id, username, email).await.unwrap();
    tx.insert_user_identity(
        Uuid::new_v4(),
        user_id,
        "password",
        &email.to_lowercase(),
        r#"{"hash":"unused"}"#,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    user_id
}

#[tokio::test]
async fn insert_and_fetch_user_by_id_and_username() {
    let storage = test_storage().await;
    let user_id = insert_user(&storage, "alice", "alice@example.com").await;

    let by_id = storage.get_user_by_id(user_id).await.unwrap().unwrap();
    assert_eq!(by_id.username, "alice");
    assert_eq!(by_id.email, "alice@example.com");
    assert!(!by_id.is_admin);

    let by_username = storage
        .get_user_by_username("alice")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(by_username.id, user_id);
}

#[tokio::test]
async fn missing_user_lookups_return_none() {
    let storage = test_storage().await;
    assert!(storage
        .get_user_by_id(Uuid::new_v4())
        .await
        .unwrap()
        .is_none());
    assert!(storage
        .get_user_by_username("nobody")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn duplicate_username_is_a_unique_violation() {
    let storage = test_storage().await;
    insert_user(&storage, "alice", "alice@example.com").await;

    let mut tx = storage.begin().await.unwrap();
    let err = tx
        .insert_user(Uuid::new_v4(), "alice", "someone-else@example.com")
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        StorageError::UniqueViolation { field: "username" }
    ));
}

#[tokio::test]
async fn duplicate_email_is_a_unique_violation() {
    let storage = test_storage().await;
    insert_user(&storage, "alice", "shared@example.com").await;

    let mut tx = storage.begin().await.unwrap();
    let err = tx
        .insert_user(Uuid::new_v4(), "bob", "shared@example.com")
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        StorageError::UniqueViolation { field: "email" }
    ));
}

#[tokio::test]
async fn dropped_transaction_rolls_back() {
    let storage = test_storage().await;
    let user_id = Uuid::new_v4();
    {
        let mut tx = storage.begin().await.unwrap();
        tx.insert_user(user_id, "alice", "alice@example.com")
            .await
            .unwrap();
        // tx dropped without commit
    }

    assert!(storage.get_user_by_id(user_id).await.unwrap().is_none());
}

#[tokio::test]
async fn user_identity_lookup_by_username_or_email() {
    let storage = test_storage().await;
    let user_id = insert_user_with_password(&storage, "alice", "Alice@Example.com").await;

    let (by_username, _) = storage
        .get_user_identity_by_login("password", "alice")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(by_username.id, user_id);

    // provider_uid is stored lowercased; login is lowercased before matching it.
    let (by_email, _) = storage
        .get_user_identity_by_login("password", "ALICE@EXAMPLE.COM")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(by_email.id, user_id);

    assert!(storage
        .get_user_identity_by_login("password", "nobody")
        .await
        .unwrap()
        .is_none());
    assert!(storage
        .get_user_identity_by_login("github", "alice")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn api_token_roundtrip() {
    let storage = test_storage().await;
    let user_id = insert_user(&storage, "alice", "alice@example.com").await;

    storage
        .insert_api_token(Uuid::new_v4(), user_id, "ci-token", "hash-abc")
        .await
        .unwrap();

    let found = storage
        .find_user_by_token_hash("hash-abc")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.id, user_id);

    assert!(storage
        .find_user_by_token_hash("does-not-exist")
        .await
        .unwrap()
        .is_none());

    let tokens = storage.list_api_tokens(user_id).await.unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].name, "ci-token");
}

#[tokio::test]
async fn repo_permission_set_get_and_remove() {
    let storage = test_storage().await;
    let owner_id = insert_user(&storage, "alice", "alice@example.com").await;
    let other_id = insert_user(&storage, "bob", "bob@example.com").await;

    let repo_id = Uuid::new_v4();
    let mut tx = storage.begin().await.unwrap();
    tx.insert_repository(repo_id, owner_id, "vault", Visibility::Private)
        .await
        .unwrap();
    tx.insert_repo_permission(repo_id, owner_id, RepoRole::Admin)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(
        storage
            .get_repo_permission(repo_id, owner_id)
            .await
            .unwrap(),
        Some(RepoRole::Admin)
    );
    assert_eq!(
        storage
            .get_repo_permission(repo_id, other_id)
            .await
            .unwrap(),
        None
    );

    // set_repo_permission upserts: grant then change role for the same user.
    storage
        .set_repo_permission(repo_id, other_id, RepoRole::Read)
        .await
        .unwrap();
    assert_eq!(
        storage
            .get_repo_permission(repo_id, other_id)
            .await
            .unwrap(),
        Some(RepoRole::Read)
    );
    storage
        .set_repo_permission(repo_id, other_id, RepoRole::Write)
        .await
        .unwrap();
    assert_eq!(
        storage
            .get_repo_permission(repo_id, other_id)
            .await
            .unwrap(),
        Some(RepoRole::Write)
    );

    storage
        .remove_repo_permission(repo_id, other_id)
        .await
        .unwrap();
    assert_eq!(
        storage
            .get_repo_permission(repo_id, other_id)
            .await
            .unwrap(),
        None
    );
}

#[tokio::test]
async fn deleting_repository_cascades_permissions() {
    let storage = test_storage().await;
    let owner_id = insert_user(&storage, "alice", "alice@example.com").await;

    let repo_id = Uuid::new_v4();
    let mut tx = storage.begin().await.unwrap();
    tx.insert_repository(repo_id, owner_id, "vault", Visibility::Private)
        .await
        .unwrap();
    tx.insert_repo_permission(repo_id, owner_id, RepoRole::Admin)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    storage.delete_repository(repo_id).await.unwrap();

    assert!(storage
        .get_repository_by_owner_and_name("alice", "vault")
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        storage
            .get_repo_permission(repo_id, owner_id)
            .await
            .unwrap(),
        None
    );
}

#[tokio::test]
async fn list_repositories_by_owner_and_for_admin() {
    let storage = test_storage().await;
    let alice_id = insert_user(&storage, "alice", "alice@example.com").await;
    let bob_id = insert_user(&storage, "bob", "bob@example.com").await;

    for (owner, name) in [(alice_id, "vault-a"), (bob_id, "vault-b")] {
        let mut tx = storage.begin().await.unwrap();
        tx.insert_repository(Uuid::new_v4(), owner, name, Visibility::Public)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }

    let alice_repos = storage.list_repositories_by_owner(alice_id).await.unwrap();
    assert_eq!(alice_repos.len(), 1);
    assert_eq!(alice_repos[0].name, "vault-a");

    let all_repos = storage.list_repositories().await.unwrap();
    assert_eq!(all_repos.len(), 2);
    assert!(all_repos
        .iter()
        .any(|(r, owner)| r.name == "vault-a" && owner == "alice"));
    assert!(all_repos
        .iter()
        .any(|(r, owner)| r.name == "vault-b" && owner == "bob"));
}

#[tokio::test]
async fn auth_session_lifecycle() {
    let storage = test_storage().await;
    let user_id = insert_user(&storage, "alice", "alice@example.com").await;

    let code = "test-session-code";
    let now = OffsetDateTime::now_utc();
    storage
        .start_auth_session(code, now + Duration::seconds(60))
        .await
        .unwrap();

    let session = storage.get_auth_session(code, now).await.unwrap().unwrap();
    assert_eq!(session.token, None);

    storage
        .complete_auth_session(
            code,
            "jwt-token",
            user_id,
            "alice",
            now + Duration::seconds(60),
        )
        .await
        .unwrap();

    let completed = storage.get_auth_session(code, now).await.unwrap().unwrap();
    assert_eq!(completed.token.as_deref(), Some("jwt-token"));
    assert_eq!(completed.user_id, Some(user_id));

    // A lookup after expiry finds nothing.
    let past_expiry = now + Duration::seconds(120);
    assert!(storage
        .get_auth_session(code, past_expiry)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn set_admin_flags_updates_both_fields() {
    let storage = test_storage().await;
    let user_id = insert_user(&storage, "alice", "alice@example.com").await;

    let mut tx = storage.begin().await.unwrap();
    tx.set_admin_flags(user_id, true, true).await.unwrap();
    tx.commit().await.unwrap();

    let user = storage.get_user_by_id(user_id).await.unwrap().unwrap();
    assert!(user.is_admin);
    assert!(user.must_change_password);
}
