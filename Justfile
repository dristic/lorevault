set windows-shell := ["pwsh", "-NoLogo", "-NoProfile", "-Command"]

# List available recipes
default:
    @just --list

# ── TLS certs ────────────────────────────────────────────────────────────────

# Generate local TLS certs for the gRPC gateway (requires mkcert)
[unix]
certs:
    mkdir -p config/certs
    mkcert -install
    mkcert -cert-file config/certs/localhost.pem -key-file config/certs/localhost-key.pem localhost 127.0.0.1 ::1

[windows]
certs:
    New-Item -ItemType Directory -Force config/certs | Out-Null
    mkcert -install
    mkcert -cert-file config/certs/localhost.pem -key-file config/certs/localhost-key.pem localhost 127.0.0.1 ::1

# ── Server ────────────────────────────────────────────────────────────────────

# Run the API + gRPC auth service (migrations run automatically on startup)
server:
    cargo run -p lv-api

# Run with debug logging for all LoreVault crates + SQL query logging
[unix]
dev:
    RUST_LOG=lv_api=debug,lv_gateway=debug,lv_auth=debug,lv_storage=debug,sqlx=debug cargo run -p lv-api

[windows]
dev:
    $env:RUST_LOG = "lv_api=debug,lv_gateway=debug,lv_auth=debug,lv_storage=debug,sqlx=debug"; cargo run -p lv-api

# Run with full trace logging (very noisy — includes tower/tonic internals)
[unix]
trace:
    RUST_LOG=trace cargo run -p lv-api

[windows]
trace:
    $env:RUST_LOG = "trace"; cargo run -p lv-api

# ── Build / test / lint ───────────────────────────────────────────────────────

build:
    cargo build

test:
    cargo test

clippy:
    cargo clippy
