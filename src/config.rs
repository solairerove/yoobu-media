// Rust key concept: no global state — config is a plain struct.
// It is created once at startup and shared via Arc<Config> with every handler.

use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub internal_api_key: String,
    pub r2_endpoint: String,
    pub r2_access_key: String,
    pub r2_secret_key: String,
    pub r2_bucket: String,
    pub r2_region: String,
    pub cdn_base_url: String,
    pub max_file_size: usize,
    pub max_image_dimension: u32,
    pub webp_quality: f32,
}

impl Config {
    // Result<T, E> is the primary error-handling pattern in Rust.
    // No exceptions, no null. The function explicitly declares what can go wrong.
    pub fn from_env() -> Result<Self, String> {
        Ok(Config {
            port: env_optional("PORT", "3000")
                .parse()
                .map_err(|e| format!("PORT: {e}"))?,

            internal_api_key: env_required("INTERNAL_API_KEY")?,

            r2_endpoint: env_required("R2_ENDPOINT")?,
            r2_access_key: env_required("R2_ACCESS_KEY")?,
            r2_secret_key: env_required("R2_SECRET_KEY")?,
            r2_bucket: env_required("R2_BUCKET")?,
            r2_region: env_optional("R2_REGION", "auto"),
            cdn_base_url: env_required("CDN_BASE_URL")?,

            max_file_size: env_optional("MAX_FILE_SIZE", "2097152")
                .parse()
                .map_err(|e| format!("MAX_FILE_SIZE: {e}"))?,

            max_image_dimension: env_optional("MAX_IMAGE_DIMENSION", "1200")
                .parse()
                .map_err(|e| format!("MAX_IMAGE_DIMENSION: {e}"))?,

            webp_quality: env_optional("WEBP_QUALITY", "80")
                .parse()
                .map_err(|e| format!("WEBP_QUALITY: {e}"))?,
        })
    }
}

fn env_required(key: &str) -> Result<String, String> {
    env::var(key).map_err(|_| format!("Missing required env var: {key}"))
}

fn env_optional(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}
