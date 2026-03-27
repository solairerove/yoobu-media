# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Check compilation without producing a binary
cargo check

# Run all tests
cargo test

# Run a single test by name (partial match works)
cargo test detect_format
cargo test auth::tests::valid_token

# Run tests in a specific module
cargo test processing::tests
cargo test auth::tests

# Build release binary
cargo build --release

# Run locally (requires env vars — copy .env.example)
INTERNAL_API_KEY=dev cargo run

# Local dev with docker (minio + image-service)
docker compose up --build

# Curl smoke test against local docker compose
curl http://localhost:3000/health
curl -X POST http://localhost:3000/upload \
  -H "Authorization: Bearer dev-internal-key-change-in-prod" \
  -H "X-Tenant-Id: 42" \
  -H "X-Upload-Path: services/17" \
  -F "file=@photo.jpg"
```

## Architecture

Single-binary Rust HTTP service. No database. No domain logic. Receives an image from Java API (internal network only), processes it, stores it in R2-compatible object storage, returns a CDN URL.

**Request flow:**

```
POST /upload
  → AuthenticatedRequest extractor  (auth.rs)   — 401 if Bearer token invalid
  → upload handler                  (handler/upload.rs)
  → detect_format()                 (processing.rs) — magic bytes, not Content-Type
  → process_image()                 (processing.rs) — decode → resize → WebP encode
  → StorageClient::upload()         (storage.rs)    — PUT to R2/MinIO via aws-sdk-s3
  → JSON { "url": "..." }
```

**Shared state:** `AppState` in `main.rs` holds `Arc<Config>` and `Arc<StorageClient>`. Cloned cheaply per request (Arc = atomic ref count, not a copy).

**Error handling:** All errors map to `AppError` enum (`error.rs`). `AppError` implements axum's `IntoResponse` — handlers return `Result<_, AppError>` and axum converts errors to JSON responses automatically.

**Auth:** `AuthenticatedRequest` is an axum `FromRequestParts` extractor. Declaring it in a handler signature makes auth mandatory — axum calls it before the handler body runs. Uses `subtle::ConstantTimeEq` to prevent timing attacks.

**Image processing:** `image` crate (0.25) decodes JPEG/PNG/WebP. `webp` crate encodes output with configurable quality. Format validated by magic bytes before decoding.

**Storage:** `StorageClient` wraps `aws-sdk-s3` configured with `force_path_style(true)` — required for both MinIO (local) and Cloudflare R2 (prod).

## Object key format

```
{tenantId}/{uploadPath}-{timestampMs}.webp

Examples:
  42/services/17-1719312000000.webp
  42/payment/qr-1719312000000.webp
```

## Environment variables

See `.env.example`. Required: `INTERNAL_API_KEY`, `R2_ENDPOINT`, `R2_ACCESS_KEY`, `R2_SECRET_KEY`, `R2_BUCKET`, `CDN_BASE_URL`. Optional with defaults: `PORT` (3000), `MAX_FILE_SIZE` (2 MB), `MAX_IMAGE_DIMENSION` (1200px), `WEBP_QUALITY` (80).

## Deployment

Railway: Dockerfile builder, service name `image-service` → internal URL `http://image-service.railway.internal:3000`. No public domain — private network only. `INTERNAL_API_KEY` is a shared Railway project variable (used by both this service and the Java API).
