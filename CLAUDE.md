# LoreVault

LoreVault is a self-hostable hosting service for [Lore](https://github.com/EpicGames/lore) repositories, built for individuals and small teams running their own instance with minimal external dependencies. Users own repositories directly; the Lore CLI connects to LoreVault just like it connects to any Lore server.

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
│ Lore protocol│     │ passwords / OAuth│
└──────┬───────┘     └──────────────────┘
       │
       ▼
┌──────────────────────────────────────────┐
│             lv-storage                   │
│  SQLite (metadata + sessions, single file)│
└──────────────────────────────────────────┘
```

VCS data (chunks, revisions) lives on the external `lore-server`; `lv-storage` only holds LoreVault's own metadata (users, tokens, repo/permission records, auth sessions).

## Crate Layout

```
crates/
  lv-core/            Core domain types, errors, pagination, traits shared across crates
  lv-auth/            Authentication (passwords, JWT, API tokens, OAuth)
  lv-storage/         Storage abstraction trait (Storage/StorageTx)
  lv-storage-sqlite/  SQLite implementation of lv-storage (sqlx)
  lv-gateway/         Tonic gRPC server implementing the auth/identity side of the Lore protocol
  lv-api/             Axum REST API: user/repo management, auth endpoints
  lv-api-types/       Wire-format request/response types shared by lv-api and lv-cli
  lv-cli/             `lorevault` CLI client for the REST API
```

## Tech Stack

| Concern          | Library                              |
|------------------|--------------------------------------|
| HTTP server      | `axum`                               |
| gRPC server      | `tonic`                              |
| Async runtime    | `tokio`                              |
| Database         | `sqlx` + SQLite                      |
| Auth             | `argon2` (passwords), `jsonwebtoken` |
| Config           | `config` crate + TOML                |
| Observability    | `tracing` + `tracing-subscriber`     |
| Errors           | `thiserror`                          |

## Data Model (high level)

```
users            id, username, email, password_hash
api_tokens       id, user_id, token_hash, scopes, expires_at
repositories     id, owner_id, name, visibility, default_branch   -- owner is always a user
repo_permissions repo_id, user_id, role (admin|write|read)
```

Repository rows are created/deleted only via lore-server's ReBAC gRPC callbacks (`lv-gateway/src/services/rebac.rs`), never directly through the REST API — that keeps LoreVault's permission metadata from drifting out of sync with the repos lore-server actually manages. `lv-api`'s repo routes are read-only.

Branches, revisions, chunks (CAS), and file locks are *not* tracked here — that VCS-level data lives entirely on the external `lore-server`; `lv-storage` only holds LoreVault's own account/permission metadata.

SSH key auth (an `ssh_keys` table) was scaffolded early on but never wired up and has been dropped; revisit if/when SSH-based auth is actually implemented.

## Lore Protocol (gRPC)

`lv-gateway` implements the **auth/identity side** of the Lore protocol that lore-server calls into — it does not implement the VCS protocol itself (repos, branches, CAS chunks, locks), which lore-server owns and serves directly:

- **UrcAuthApi** (`proto/auth_api.proto`) — exchanges API keys/external tokens for short-lived JWTs, mints multi-resource-scoped tokens, resolves per-repo permissions (`CheckUserPermission`), and drives the browser-based CLI login flow (`StartAuthSession` / `GetAuthSession`). Several methods (`RefreshAuthSession`, `VerifyUser`, `LookupUserPermissions`, `GetUserInfo`, `GetUserId`, `GetProviderUserId`) are still stubbed as `unimplemented`.
- **RebacApi** (`proto/rebac_api.proto`) — lore-server calls back into this on repo create/delete so LoreVault's `repositories`/`repo_permissions` tables stay in sync with what lore-server actually manages (see `lv-gateway/src/services/rebac.rs`).
- **EnvironmentService** (`proto/environment.proto` and `proto/lore/environment/v1/environment.proto` — legacy and current wire versions) — tells the Lore CLI which `auth_url` to use.

Most calls require an `Authorization: Bearer <jwt>` metadata header (validated via `extract_claims`), the exception being the initial API-key exchange, which carries the raw API key in the request body since the caller doesn't have a JWT yet.

## Local Development

Requires: `cargo`, `sqlx-cli`

```bash
# Run migrations (creates ./data/lorevault.db if it doesn't exist)
sqlx migrate run --database-url sqlite:./data/lorevault.db

# Run the server
cargo run -p lv-api
```

No external services (Postgres, Redis, MinIO) are required — LoreVault runs against a single SQLite file, matching the goal of an easy-to-self-host, low-dependency deployment.

Environment is configured via `config/local.toml` (see `config/default.toml` for all keys).

## Key Conventions

- All database queries use `sqlx` compile-time checked macros (`query!`, `query_as!`).
- Secrets (tokens, passwords) are **never** stored plaintext; use Argon2 for passwords, SHA-256 for token hashes.
- Ownership is enforced at the storage layer: every query includes `repo_id` or `owner_id`, never relying on application-level filtering alone.
- gRPC errors map to `tonic::Status`; REST errors serialize to `{ "error": "...", "code": "..." }`.
- `tracing::instrument` on every public async fn in service layers.
