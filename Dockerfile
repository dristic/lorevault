# ── Build stage ──────────────────────────────────────────────────────────────
FROM rust:1.88-bookworm AS builder

RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace manifests first so dependency layers cache independently
COPY Cargo.toml Cargo.lock ./
COPY crates/lv-core/Cargo.toml   crates/lv-core/
COPY crates/lv-auth/Cargo.toml   crates/lv-auth/
COPY crates/lv-storage/Cargo.toml crates/lv-storage/
COPY crates/lv-storage-sqlite/Cargo.toml crates/lv-storage-sqlite/
COPY crates/lv-gateway/Cargo.toml crates/lv-gateway/
COPY crates/lv-api/Cargo.toml    crates/lv-api/

# Stub all lib/main targets so cargo can fetch and build deps
RUN for crate in lv-core lv-auth lv-storage lv-storage-sqlite lv-gateway; do \
      mkdir -p crates/$crate/src && printf 'pub fn _stub() {}' > crates/$crate/src/lib.rs; \
    done && \
    mkdir -p crates/lv-api/src && printf 'fn main() {}' > crates/lv-api/src/main.rs

# Stub build.rs so tonic-build runs without real protos
RUN printf 'fn main() {}' > crates/lv-gateway/build.rs

RUN cargo build --release 2>/dev/null || true

# Now copy the real source and rebuild only what changed
COPY proto              proto
COPY crates             crates
COPY migrations         migrations

RUN find crates proto -type f \( -name '*.rs' -o -name '*.proto' \) -exec touch {} + && \
    cargo build --release -p lv-api

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/lorevault /usr/local/bin/lorevault
# Config directory must be present; secrets come from env vars at runtime
COPY config /app/config

EXPOSE 3000 9001

CMD ["lorevault"]
