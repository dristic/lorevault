# LoreVault

A self-hostable Git-style hosting service for [Lore](https://github.com/EpicGames/lore) repositories. LoreVault adds user accounts, organizations, multi-tenant repository management, and access control on top of the Lore VCS protocol — the same way GitHub wraps Git.

> **Status:** Early development. Core scaffold is in place; gRPC gateway and full auth middleware are in progress.

---

## What is Lore?

Lore is Epic Games' open-source version control system optimized for large binary assets (games, media). It stores repository state as Merkle trees with content-addressed, chunked storage and an immutable revision chain — designed to handle codebases that mix source code with gigabytes of binary data.

LoreVault lets teams host Lore repositories centrally, just as GitHub hosts Git repositories. The Lore CLI connects to a LoreVault instance as its remote server.

---

## Architecture

```
Clients (Lore CLI / Browser / REST API consumers)
        │ gRPC (Lore protocol)    │ HTTPS REST
        ▼                         ▼
┌────────────────────────────────────────────────┐
│                  lv-api  (Axum)                │
│      REST management API + auth endpoints      │
└───────────────────┬────────────────────────────┘
                    │
        ┌───────────┴───────────┐
        │                       │
        ▼                       ▼
┌──────────────┐      ┌──────────────────┐
│  lv-gateway  │      │    lv-auth       │
│ (Tonic gRPC) │      │ JWT, API tokens  │
│ Lore protocol│      │ SSH keys, OAuth  │
└──────┬───────┘      └──────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────┐
│                  lv-storage                      │
│  PostgreSQL (metadata) │ S3/MinIO (blob/CAS)     │
│  Redis (sessions, distributed locks, cache)      │
└──────────────────────────────────────────────────┘
```

### Crates

| Crate | Description |
|---|---|
| `lv-core` | Domain models: `User`, `Organization`, `Repository`, `Branch`, `Revision`, `Chunk`, `FileLock` |
| `lv-auth` | Password hashing (Argon2), JWT session tokens, SHA-256 API token generation |
| `lv-storage` | PostgreSQL pool (sqlx), S3/MinIO blob store, Redis connection pool |
| `lv-gateway` | Tonic gRPC server implementing the Lore wire protocol (RepoService, CasService, LockService, AdminService) |
| `lv-api` | Axum HTTP server: user/org/repo management REST API, auth endpoints |
| `lv-worker` | Background jobs: CAS garbage collection, webhook delivery, token expiry cleanup |

---

## Tech Stack

| Concern | Library |
|---|---|
| HTTP server | `axum` |
| gRPC server | `tonic` |
| Async runtime | `tokio` |
| Database | `sqlx` + PostgreSQL 16 |
| Cache / sessions | `deadpool-redis` + Redis 7 |
| Object storage | `aws-sdk-s3` (MinIO for local dev) |
| Passwords | `argon2` |
| Sessions | `jsonwebtoken` (HS256) |
| Config | `config` crate + TOML |
| Observability | `tracing` + `tracing-subscriber` |

---

## Data Model

```
users            id, username, email, password_hash
organizations    id, slug, display_name
org_members      org_id, user_id, role (owner | admin | member)
api_tokens       id, user_id, token_hash, scopes, expires_at
ssh_keys         id, user_id, fingerprint, public_key
repositories     id, owner_type, owner_id, name, visibility, default_branch
repo_permissions repo_id, user_id, role (admin | write | read)
branches         id, repo_id, name, head_revision_hash
revisions        id, repo_id, hash, parent_hashes[], author, message, timestamp
chunks           id, repo_id, hash, size_bytes, storage_key   ← CAS index
file_locks       id, repo_id, path, locked_by_user_id, workspace_id
```

Repositories are namespaced as `{owner}/{repo}` (where `owner` is a username or org slug), mirroring the GitHub URL convention.

---

## Lore gRPC Protocol

The Lore CLI speaks gRPC to its server. LoreVault implements the same services:

- **RepoService** — branch CRUD, revision read/write
- **CasService** — chunk find-missing, upload, download (content-addressed storage)
- **LockService** — acquire/release/query exclusive file locks
- **AdminService** — tenant-scoped administration

Every gRPC call carries `Authorization: Bearer <api_token>` in the metadata. The gateway validates the token, resolves the tenant from the request, checks permissions, then routes to the appropriate storage namespace.

---

## Authentication

| Method | Use case |
|---|---|
| Username + password → JWT | Web sessions / browser |
| API token (`lv_…`) | Lore CLI, automation, CI |
| SSH key *(planned)* | Alternative CLI auth |
| OAuth *(planned)* | SSO via GitHub / Google |

API tokens are stored as SHA-256 hashes — the raw value is shown only once at creation time.

---

## Local Development

**Prerequisites:** `cargo`, `docker compose`, `sqlx-cli`

```bash
# 1. Start dependencies (Postgres, Redis, MinIO)
docker compose -f docker/docker-compose.yml up -d

# 2. Run database migrations
sqlx migrate run --database-url postgres://lorevault:lorevault@localhost/lorevault

# 3. Start the server
cargo run -p lv-api
```

The REST API is available at `http://localhost:3000` and the gRPC gateway at `localhost:50051`.

Configuration is loaded from `config/default.toml`. Override locally by creating `config/local.toml` (git-ignored) or via environment variables prefixed with `LV__` (e.g. `LV__AUTH__JWT_SECRET=...`).

### MinIO console

The local MinIO instance exposes a web UI at `http://localhost:9001` (credentials: `lorevault` / `lorevault123`).

---

## REST API (v1)

| Method | Path | Description |
|---|---|---|
| `GET` | `/healthz` | Health check (includes DB ping) |
| `POST` | `/api/v1/auth/register` | Create a new user account |
| `POST` | `/api/v1/auth/login` | Authenticate and receive a JWT |
| `POST` | `/api/v1/auth/tokens` | Create an API token |
| `GET` | `/api/v1/users/:username` | Get user profile |
| `POST` | `/api/v1/repos` | Create a repository |
| `GET` | `/api/v1/repos/:owner/:repo` | Get repository details |

Errors are returned as `{ "error": "...", "code": "..." }` with appropriate HTTP status codes.

---

## Roadmap

- [ ] Auth middleware extractor (JWT → `AppState` user context)
- [ ] Organization management endpoints
- [ ] Repository visibility enforcement (public/private access checks)
- [ ] Vendor Lore `.proto` files and generate Tonic stubs
- [ ] Implement `CasService` (chunk upload/download backed by S3)
- [ ] Implement `RepoService` (branch + revision operations)
- [ ] Implement `LockService`
- [ ] `cargo sqlx prepare` offline query cache
- [ ] SSH key authentication
- [ ] Webhook delivery via `lv-worker`
- [ ] Web UI

---

## License

MIT
