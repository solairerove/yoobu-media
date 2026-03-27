// Rust key concept: axum extractors in the function signature.
// Axum calls from_request_parts / from_request for each argument
// in the correct order, verifying types statically at compile time.
//
// State(state) — shared state (Arc under the hood).
// _auth: AuthenticatedRequest — if extraction fails → 401, handler body never runs.
// HeaderMap — raw request headers.
// Multipart — must be last because it consumes the request body.

use axum::{
    extract::{Multipart, State},
    http::HeaderMap,
    Json,
};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{auth::AuthenticatedRequest, error::AppError, processing, AppState};

pub async fn upload(
    State(state): State<AppState>,
    _auth: AuthenticatedRequest,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<Value>, AppError> {
    let tenant_id = required_header(&headers, "X-Tenant-Id")?;
    let upload_path = required_header(&headers, "X-Upload-Path")?;

    let file_bytes = extract_file(&mut multipart).await?;

    if file_bytes.len() > state.config.max_file_size {
        return Err(AppError::BadRequest("File exceeds 2 MB limit".into()));
    }

    // Validate format via magic bytes before decoding
    processing::detect_format(&file_bytes)?;

    let webp_bytes = processing::process_image(
        &file_bytes,
        state.config.max_image_dimension,
        state.config.webp_quality,
    )?;

    // {tenantId}/{uploadPath}-{timestampMs}.webp
    // Example: 42/services/17-1719312000000.webp
    let key = build_object_key(&tenant_id, &upload_path);

    tracing::info!(key = key, size_bytes = webp_bytes.len(), "Uploading to R2");
    state.storage.upload(&key, webp_bytes).await?;

    let url = format!(
        "{}/{}",
        state.config.cdn_base_url.trim_end_matches('/'),
        key
    );

    Ok(Json(json!({ "url": url })))
}

fn required_header(headers: &HeaderMap, name: &str) -> Result<String, AppError> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest(format!("Missing header: {name}")))
}

fn build_object_key(tenant_id: &str, upload_path: &str) -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before UNIX epoch")
        .as_millis();

    format!("{tenant_id}/{upload_path}-{ts}.webp")
}

async fn extract_file(multipart: &mut Multipart) -> Result<Vec<u8>, AppError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        if field.name() == Some("file") {
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(e.to_string()))?;
            return Ok(data.to_vec());
        }
    }

    Err(AppError::BadRequest(
        "Multipart field 'file' not found".into(),
    ))
}
