set windows-shell := ["pwsh", "-NoLogo", "-NoProfile", "-Command"]

# List available recipes
default:
    @just --list

# ── Dependencies ──────────────────────────────────────────────────────────────

# Start Postgres and Redis via Docker Compose
deps:
    podman compose -f docker/docker-compose.yml up -d

# Stop and remove Docker containers
deps-down:
    podman compose -f docker/docker-compose.yml down

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

# ── Local dev setup ───────────────────────────────────────────────────────────

# Register a dev user, create an API token and a test repo.
# Requires: `just deps` running and `just server` running in another terminal.

[unix]
setup username="dev" email="dev@example.com" password="password" repo="test-repo" url="http://localhost:3000":
    #!/usr/bin/env python3
    import json, sys, urllib.request, urllib.error, time

    base = "{{url}}"

    def post(path, body, token=None):
        headers = {"Content-Type": "application/json"}
        if token:
            headers["Authorization"] = f"Bearer {token}"
        req = urllib.request.Request(
            base + path, data=json.dumps(body).encode(), headers=headers
        )
        try:
            with urllib.request.urlopen(req) as r:
                return json.load(r)
        except urllib.error.HTTPError as e:
            return json.load(e)

    print("Waiting for server...", flush=True)
    for _ in range(30):
        try:
            urllib.request.urlopen(base + "/healthz")
            break
        except Exception:
            time.sleep(1)
    else:
        print("Server did not become ready after 30 seconds", file=sys.stderr)
        sys.exit(1)

    print("Registering user (or logging in if already exists)...", flush=True)
    auth = post("/api/v1/auth/register", {
        "username": "{{username}}",
        "email": "{{email}}",
        "password": "{{password}}",
    })
    if "token" not in auth:
        auth = post("/api/v1/auth/login", {
            "login": "{{username}}",
            "password": "{{password}}",
        })
    if "token" not in auth:
        print(f"Error: {auth}", file=sys.stderr)
        sys.exit(1)

    jwt = auth["token"]
    user_id = auth["user_id"]

    print("Creating API token...", flush=True)
    tok = post("/api/v1/auth/tokens", {"name": "dev-cli"}, jwt)

    print("Creating repository...", flush=True)
    post("/api/v1/repos", {
        "owner_id": user_id,
        "owner_type": "user",
        "name": "{{repo}}",
        "visibility": "private",
    }, jwt)

    print()
    print("Done! Authenticate the Lore CLI:")
    print()
    print(f"  lore auth login grpcs://localhost:41337 --token-type api-key --token {tok['token']}")
    print()
    print("Or use the browser login flow:")
    print()
    print("  lore auth login grpcs://localhost:41337")

[windows]
setup username="dev" email="dev@example.com" password="password" repo="test-repo" url="http://localhost:3000":
    pwsh -NoLogo -NoProfile -File scripts/setup.ps1 -Username "{{username}}" -Email "{{email}}" -Password "{{password}}" -Repo "{{repo}}" -Url "{{url}}"
