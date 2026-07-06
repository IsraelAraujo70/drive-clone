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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedUploadPart {
    pub part_number: i32,
    pub etag: String,
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

    async fn create_multipart_upload(
        &self,
        object_key: &str,
        content_type: &str,
    ) -> Result<String, StorageError>;

    async fn presign_upload_part(
        &self,
        object_key: &str,
        upload_id: &str,
        part_number: i32,
        ttl_seconds: i64,
    ) -> Result<PresignedUrl, StorageError>;

    async fn complete_multipart_upload(
        &self,
        object_key: &str,
        upload_id: &str,
        parts: &[CompletedUploadPart],
    ) -> Result<(), StorageError>;

    async fn abort_multipart_upload(
        &self,
        object_key: &str,
        upload_id: &str,
    ) -> Result<(), StorageError>;
}
