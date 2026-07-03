use async_trait::async_trait;

use crate::application::ports::StorageError;
use crate::application::ports::object_storage::{ObjectMetadata, ObjectStorage, PresignedUrl};

#[derive(Debug, Default)]
pub struct DisabledObjectStorage;

#[async_trait]
impl ObjectStorage for DisabledObjectStorage {
    async fn presign_put(
        &self,
        _object_key: &str,
        _content_type: &str,
        _size_bytes: i64,
        _ttl_seconds: i64,
    ) -> Result<PresignedUrl, StorageError> {
        Err(StorageError::Unexpected)
    }

    async fn presign_get(
        &self,
        _object_key: &str,
        _ttl_seconds: i64,
    ) -> Result<PresignedUrl, StorageError> {
        Err(StorageError::Unexpected)
    }

    async fn head_object(&self, _object_key: &str) -> Result<ObjectMetadata, StorageError> {
        Err(StorageError::Unexpected)
    }
}
