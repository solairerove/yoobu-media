// Rust key concept: errors are types, not exceptions.
// thiserror::Error generates impl std::error::Error and Display automatically.
//
// IntoResponse is an axum trait. By implementing it for AppError we tell axum:
// "this error can be returned from a handler as an HTTP response".
// Axum calls into_response() automatically when a handler returns Err(AppError).

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Unauthorized")]
    Unauthorized,

    #[error("{0}")]
    BadRequest(String),

    #[error("Storage upload failed")]
    StorageError,

    #[error("Image processing failed: {0}")]
    ProcessingError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::StorageError => (StatusCode::BAD_GATEWAY, self.to_string()),
            AppError::ProcessingError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
