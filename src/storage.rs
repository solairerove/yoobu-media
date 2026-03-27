// Rust key concept: async/await without a GC.
// StorageClient holds a Client from aws-sdk — it is thread-safe (Client: Clone + Send + Sync).
// In Rust this is guaranteed by the compiler via trait bounds, not runtime checks.

use aws_credential_types::Credentials;
use aws_sdk_s3::{
    config::{BehaviorVersion, Region},
    primitives::ByteStream,
    Client,
};
use bytes::Bytes;

use crate::{config::Config, error::AppError};

pub struct StorageClient {
    client: Client,
    bucket: String,
}

impl StorageClient {
    pub fn new(config: &Config) -> Self {
        let credentials = Credentials::new(
            &config.r2_access_key,
            &config.r2_secret_key,
            None, // session token
            None, // expiry
            "static",
        );

        let s3_config = aws_sdk_s3::config::Builder::new()
            .behavior_version(BehaviorVersion::latest())
            .endpoint_url(&config.r2_endpoint)
            // path-style: http://host/bucket/key
            // virtual-hosted: http://bucket.host/key
            // Both R2 and MinIO require path-style
            .force_path_style(true)
            .credentials_provider(credentials)
            .region(Region::new(config.r2_region.clone()))
            .build();

        StorageClient {
            client: Client::from_conf(s3_config),
            bucket: config.r2_bucket.clone(),
        }
    }

    pub async fn upload(&self, key: &str, data: Vec<u8>) -> Result<(), AppError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(Bytes::from(data)))
            .content_type("image/webp")
            .cache_control("public, max-age=31536000, immutable")
            .send()
            .await
            .map_err(|e| {
                tracing::error!(key = key, error = ?e, "R2 upload failed");
                AppError::StorageError
            })?;

        Ok(())
    }

    /// Deletes an object. S3/R2 delete is idempotent — returns 204 even if the object did not exist.
    /// We propagate an error only on a network/auth failure, never on "not found".
    pub async fn delete(&self, key: &str) -> Result<(), AppError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                tracing::error!(key = key, error = ?e, "R2 delete failed");
                AppError::StorageError
            })?;

        Ok(())
    }
}
