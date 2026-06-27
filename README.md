# LoreVault

A self-hostable storage and authentication service for [Lore](https://github.com/EpicGames/lore) repositories. LoreVault adds user accounts, organizations, repository management, and access control on top of the Lore VCS protocol.

> **Status:** Early development. Core scaffold is in place; gRPC gateway and full auth middleware are in progress.

---

## Getting Started

The easiest way to get started is to build the docker container and run it locally:

```bash
docker build -t lorevault:v1 .
```

This will build the project in a container with the rust toolchain then build a lightweight app container to run the project.

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

---

## Contributing

If you are interested in contributing to LoreVault read through the [CONTRIBUTING.md](CONTRIBUTING.md).

---

## License

LoreVault is dual licensed under the AGPL 3.0 or a Commercial license. For more information visit [lorevault.io](https://lorevault.io)
