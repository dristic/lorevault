# LoreVault

LoreVault is a multi-tenant hosting service for [Lore](https://github.com/EpicGames/lore) repositories — think GitHub, but for Lore VCS. Users and organizations own repositories; the Lore CLI connects to LoreVault just like it connects to any Lore server.

## Architecture Overview

```
Clients (Lore CLI / Browser / API consumers)
        │ gRPC                │ HTTPS REST
        ▼                     ▼
┌─────────────────────────────────────────┐
│           lv-api  (Axum)                │
│  REST management API + auth endpoints   │
└──────────────────┬──────────────────────┘
                   │
       ┌───────────┴───────────┐
       │                       │
       ▼                       ▼
┌──────────────┐     ┌──────────────────┐
│ lv-gateway   │     │   lv-auth        │
│ (Tonic gRPC) │     │ JWT + API tokens │
│ Lore protocol│     │ SSH keys / OAuth │
└──────┬───────┘     └──────────────────┘
       │
       ▼
┌──────────────────────────────────────────┐
│             lv-storage                   │
│  PostgreSQL (metadata) │ S3/MinIO (blobs)│
│  Redis (sessions, locks, cache)          │
└──────────────────────────────────────────┘
```

## Crate Layout

```
crates/
  lv-core/        Core domain types, errors, traits shared across crates
  lv-auth/        Authentication (passwords, JWT, API tokens, SSH, OAuth)
  lv-storage/     Storage abstraction: Postgres (sqlx) + S3 + Redis
  lv-gateway/     Tonic gRPC server implementing the Lore protocol
  lv-api/         Axum REST API: user/org/repo management, web hooks
  lv-worker/      Background jobs (GC, webhooks, notifications)
```

## Tech Stack

| Concern          | Library                              |
|------------------|--------------------------------------|
| HTTP server      | `axum`                               |
| gRPC server      | `tonic`                              |
| Async runtime    | `tokio`                              |
| Database         | `sqlx` + PostgreSQL 16               |
| Cache / sessions | `deadpool-redis` + Redis 7           |
| Object storage   | `aws-sdk-s3` (MinIO for local dev)   |
| Auth             | `argon2` (passwords), `jsonwebtoken` |
| Config           | `config` crate + TOML                |
| Observability    | `tracing` + `tracing-subscriber`     |
| Errors           | `thiserror`                          |

## Data Model (high level)

```
users            id, username, email, password_hash
organizations    id, slug, display_name
org_members      org_id, user_id, role (owner|admin|member)
api_tokens       id, user_id, token_hash, scopes, expires_at
ssh_keys         id, user_id, fingerprint, public_key
repositories     id, owner_type, owner_id, name, visibility, default_branch
repo_permissions repo_id, user_id, role (admin|write|read)
branches         id, repo_id, name, head_revision_hash
revisions        id, repo_id, hash, parent_hashes, author, message, timestamp
chunks           id, repo_id, hash, size, storage_key   -- CAS index
file_locks       id, repo_id, path, locked_by_user_id, workspace_id
```

## Lore Protocol (gRPC)

Lore's wire protocol is gRPC (protobuf). The services we must implement:

- **RepoService** — create/read/list/delete repos, branch CRUD, revision write/read
- **CasService** — chunk upload (find-missing, upload), chunk download
- **LockService** — acquire/release/query exclusive file locks
- **AdminService** — server admin operations (tenant scoped in LoreVault)

Each gRPC call carries an `Authorization: Bearer <api_token>` metadata header. The gateway validates the token, resolves the tenant (owner/repo from request), checks permissions, then routes to the right storage namespace.

## Local Development

Requires: `cargo`, `docker compose`, `sqlx-cli`

```bash
# Start dependencies
docker compose up -d

# Run migrations
sqlx migrate run --database-url postgres://lorevault:lorevault@localhost/lorevault

# Run the server
cargo run -p lv-api
```

Environment is configured via `config/local.toml` (see `config/default.toml` for all keys).

## Key Conventions

- All database queries use `sqlx` compile-time checked macros (`query!`, `query_as!`).
- Secrets (tokens, passwords) are **never** stored plaintext; use Argon2 for passwords, SHA-256 for token hashes.
- Multi-tenancy is enforced at the storage layer: every query includes `repo_id` or `owner_id`, never relying on application-level filtering alone.
- gRPC errors map to `tonic::Status`; REST errors serialize to `{ "error": "...", "code": "..." }`.
- `tracing::instrument` on every public async fn in service layers.
