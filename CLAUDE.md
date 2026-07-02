# LoreVault

LoreVault is a self-hostable hosting service for [Lore](https://github.com/EpicGames/lore) repositories, built for individuals and small teams running their own instance with minimal external dependencies. Users own repositories directly; the Lore CLI connects to LoreVault just like it connects to any Lore server.

> **Note:** the schema/models still have organization support (`organizations`, `org_members`, `OwnerType::Org`) left over from an earlier multi-tenant design. That's slated for removal but hasn't happened yet — don't design new features around orgs, and expect this section to simplify further once that cleanup lands.

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
│  SQLite (metadata + sessions, single file)│
└──────────────────────────────────────────┘
```

VCS data (chunks, revisions) lives on the external `lore-server`; `lv-storage` only holds LoreVault's own metadata (users, tokens, repo/permission records, auth sessions).

## Crate Layout

```
crates/
  lv-core/        Core domain types, errors, traits shared across crates
  lv-auth/        Authentication (passwords, JWT, API tokens, SSH, OAuth)
  lv-storage/     Storage abstraction: SQLite (sqlx)
  lv-gateway/     Tonic gRPC server implementing the Lore protocol
  lv-api/         Axum REST API: user/repo management, web hooks
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
ssh_keys         id, user_id, fingerprint, public_key
repositories     id, owner_id, name, visibility, default_branch   -- owner is always a user; owner_type/org columns are legacy, being removed
repo_permissions repo_id, user_id, role (admin|write|read)
```

Branches, revisions, chunks (CAS), and file locks are *not* tracked here — that VCS-level data lives entirely on the external `lore-server`; `lv-storage` only holds LoreVault's own account/permission metadata.

## Lore Protocol (gRPC)

Lore's wire protocol is gRPC (protobuf). The services we must implement:

- **RepoService** — create/read/list/delete repos, branch CRUD, revision write/read
- **CasService** — chunk upload (find-missing, upload), chunk download
- **LockService** — acquire/release/query exclusive file locks
- **AdminService** — server admin operations

Each gRPC call carries an `Authorization: Bearer <api_token>` metadata header. The gateway validates the token, resolves the owner/repo from the request, and checks permissions before routing to the right storage namespace.

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
