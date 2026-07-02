# LoreVault

A self-hostable storage and authentication service for [Lore](https://github.com/EpicGames/lore) repositories. LoreVault adds user accounts, repository management, and access control on top of the Lore VCS protocol.

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

**Prerequisites:** `cargo`, `sqlx-cli`

```bash
# 1. Run database migrations (creates ./data/lorevault.db if it doesn't exist)
sqlx migrate run --database-url sqlite:./data/lorevault.db

# 2. Start the server
cargo run -p lv-api
```

No external services are required — LoreVault runs against a single SQLite file. The REST API is available at `http://localhost:3000` and the gRPC gateway at `localhost:9001`.

Configuration is loaded from `config/default.toml`. Override locally by creating `config/local.toml` (git-ignored) or via environment variables prefixed with `LV__` (e.g. `LV__AUTH__JWT_SECRET=...`).

---

## Contributing

If you are interested in contributing to LoreVault read through the [CONTRIBUTING.md](CONTRIBUTING.md).

---

## License

LoreVault is dual licensed under the AGPL 3.0 or a Commercial license. For more information visit [lorevault.io](https://lorevault.io)
