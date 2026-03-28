# ── Stage 1: Builder ─────────────────────────────────────────────────────────
# rust:slim — Debian slim + Rust toolchain.
# Includes gcc and make required to compile C code (libwebp bundled by the webp crate).
FROM rust:slim-bookworm AS builder

WORKDIR /app

# Copy only manifests and build a stub main.rs first.
# Docker caches this layer — when only src/ changes, dependencies are not recompiled.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

# Now copy the real sources and rebuild only them.
COPY src/ src/
# touch so cargo detects a timestamp change after the stub build.
RUN touch src/main.rs && cargo build --release

# ── Stage 2: Runtime ──────────────────────────────────────────────────────────
# debian:bookworm-slim — minimal runtime image (~30 MB vs ~1.5 GB builder).
# Rust produces a mostly-static binary.
# Only ca-certificates is needed for TLS connections to R2.
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/yoobu-media /usr/local/bin/yoobu-media

EXPOSE 3000

CMD ["yoobu-media"]
