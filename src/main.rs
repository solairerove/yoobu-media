// Rust key concept: Arc<T> — Atomically Reference Counted.
// Shared ownership across threads without a mutex.
// Arc<Config> is cloned cheaply (atomic ref-count increment); the data is not copied.
// When the last Arc is dropped, Config is freed.
//
// In Java everything on the heap is shared by default (GC handles lifetimes).
// In Rust ownership is explicit: you decide when and how to share.

use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, post},
    Router,
};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod auth;
mod config;
mod error;
mod handler;
mod processing;
mod storage;

use config::Config;
use storage::StorageClient;

/// Shared state passed to every handler.
/// Clone is cheap — only the Arc is cloned (atomic ref-count increment, no data copy).
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub storage: Arc<StorageClient>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "yoobu_media=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env().unwrap_or_else(|e| {
        eprintln!("Configuration error: {e}");
        std::process::exit(1);
    });

    let port = config.port;
    let max_body = config.max_file_size + 64 * 1024; // extra headroom for multipart headers

    let storage = StorageClient::new(&config);

    let state = AppState {
        config: Arc::new(config),
        storage: Arc::new(storage),
    };

    let app = Router::new()
        .route("/health", get(handler::health::health))
        .route("/upload", post(handler::upload::upload))
        .route("/object", delete(handler::delete::delete))
        .layer(DefaultBodyLimit::max(max_body))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to bind to port {port}: {e}");
            std::process::exit(1);
        });

    tracing::info!("yoobu-media listening on port {port}");

    axum::serve(listener, app).await.unwrap();
}
