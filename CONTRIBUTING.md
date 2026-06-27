# Contributing to LoreVault

## Contributor License Agreement

**You must sign the CLA before your pull request can be merged.**

LoreVault is dual-licensed under AGPL-3.0 (open source) and a commercial license (for hosted
deployments). To preserve the ability to offer both licenses, we need contributors to grant
LoreVault the right to distribute their contributions under either license.

By signing, you:
- Retain full copyright over your contribution
- Grant LoreVault a perpetual, irrevocable license to distribute your contribution under AGPL-3.0
- Grant LoreVault a perpetual, irrevocable license to distribute your contribution under
  commercial terms

**Sign the CLA**: [CLA link TBD — will be added before the first external PR is accepted]

If you are contributing on behalf of an employer, a corporate CLA is required. Contact
hello@lorevault.dev.

## Development Setup

```bash
# Prerequisites: cargo, docker compose, sqlx-cli
docker compose up -d
sqlx migrate run --database-url postgres://lorevault:lorevault@localhost/lorevault
cargo run -p lv-api
```

See `config/default.toml` for all configuration keys. Copy to `config/local.toml` and override
as needed.

## Code Style

- `cargo clippy -- -D warnings` must pass (enforced in CI)
- `cargo fmt` before every commit
- No `unwrap()` or `expect()` in library code — propagate errors with `?`
- `tracing::instrument` on every public async function in service layers
- Compile-time SQL checking via `sqlx` macros — run `cargo sqlx prepare` if you change queries

## Pull Request Guidelines

- One logical change per PR
- Include a migration if your change touches the database schema
- Tests for new query paths; integration tests preferred over unit mocks for DB code
- Reference any related issue in the PR description
