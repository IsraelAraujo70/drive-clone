use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::application::ports::StorageError;

#[derive(Debug, Clone)]
pub struct PresignedUrl {
    pub url: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ObjectMetadata {
    pub content_length: i64,
}

#[async_trait]
pub trait ObjectStorage: Send + Sync {
    async fn presign_put(
        &self,
        object_key: &str,
        content_type: &str,
        size_bytes: i64,
        ttl_seconds: i64,
    ) -> Result<PresignedUrl, StorageError>;

    async fn presign_get(
        &self,
        object_key: &str,
        ttl_seconds: i64,
    ) -> Result<PresignedUrl, StorageError>;

    async fn head_object(&self, object_key: &str) -> Result<ObjectMetadata, StorageError>;
}
