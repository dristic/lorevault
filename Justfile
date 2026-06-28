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

# ── Database ──────────────────────────────────────────────────────────────────

# Run pending sqlx migrations against the local database
migrate:
    sqlx migrate run --database-url postgres://lorevault:lorevault@localhost/lorevault

# ── Server ────────────────────────────────────────────────────────────────────

# Run the API + gRPC gateway (start `just deps` first; run this in its own terminal)
server:
    cargo run -p lv-api

# ── Build / test / lint ───────────────────────────────────────────────────────

build:
    cargo build

test:
    cargo test

clippy:
    cargo clippy
