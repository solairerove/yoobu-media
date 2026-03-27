use axum::{extract::State, http::HeaderMap, http::StatusCode};

use crate::{auth::AuthenticatedRequest, error::AppError, AppState};

pub async fn delete(
    State(state): State<AppState>,
    _auth: AuthenticatedRequest,
    headers: HeaderMap,
) -> Result<StatusCode, AppError> {
    let key = headers
        .get("X-Object-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Missing header: X-Object-Key".into()))?
        .to_string();

    tracing::info!(key = key, "Deleting object from R2");
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}
