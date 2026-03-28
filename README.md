# yoobu-media

Image service for yoobu. Accepts multipart image upload, converts to WebP, stores in R2-compatible object storage, returns a CDN URL.

---

## Local development

### 1. Start storage only (MinIO)

```bash
docker compose up minio minio-init
```

MinIO S3 API is available at `http://localhost:9000`, web console at `http://localhost:9001` (minioadmin / minioadmin).

### 2. Create a local `.env`

```bash
cp .env.example .env
```

For local development with MinIO the `.env` file looks like this:

```env
PORT=3000
INTERNAL_API_KEY=dev-internal-key

R2_ENDPOINT=http://localhost:9000
R2_ACCESS_KEY=minioadmin
R2_SECRET_KEY=minioadmin
R2_BUCKET=yoobu-media
R2_REGION=auto
CDN_BASE_URL=http://localhost:9000/yoobu-media

RUST_LOG=image_service=debug,tower_http=debug
```

### 3. Run the service locally

```bash
source .env && cargo run
```

Or without polluting the current shell:

```bash
env $(grep -v '^#' .env | grep -v '^$' | xargs) cargo run
```

The service starts on `http://localhost:3000`.

---

## Smoke test

```bash
# Health check
curl http://localhost:3000/health

# Upload
curl -X POST http://localhost:3000/upload \
  -H "Authorization: Bearer dev-internal-key" \
  -H "X-Tenant-Id: 42" \
  -H "X-Upload-Path: services/17" \
  -F "file=@/path/to/image.jpg"

# Delete
curl -X DELETE http://localhost:3000/object \
  -H "Authorization: Bearer dev-internal-key" \
  -H "X-Object-Key: 42/services/17-1719312000000.webp"
```

---

## Tests

```bash
# All tests
cargo test

# Single test by name (partial match)
cargo test detect_format
cargo test auth::tests
```

---

## Full stack (docker compose)

To run everything together uncomment the `yoobu-media` block in `docker-compose.yml`:

```bash
docker compose up --build
```

claude --resume d11dc0c1-0e6a-435c-a679-2467a6510deb