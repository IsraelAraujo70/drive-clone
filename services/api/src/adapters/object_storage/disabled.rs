use async_trait::async_trait;

use crate::application::ports::StorageError;
use crate::application::ports::object_storage::{
    CompletedUploadPart, ObjectMetadata, ObjectStorage, PresignedUrl, StoredObject,
};

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

    async fn create_multipart_upload(
        &self,
        _object_key: &str,
        _content_type: &str,
    ) -> Result<String, StorageError> {
        Err(StorageError::Unexpected)
    }

    async fn presign_upload_part(
        &self,
        _object_key: &str,
        _upload_id: &str,
        _part_number: i32,
        _ttl_seconds: i64,
    ) -> Result<PresignedUrl, StorageError> {
        Err(StorageError::Unexpected)
    }

    async fn complete_multipart_upload(
        &self,
        _object_key: &str,
        _upload_id: &str,
        _parts: &[CompletedUploadPart],
    ) -> Result<(), StorageError> {
        Err(StorageError::Unexpected)
    }

    async fn abort_multipart_upload(
        &self,
        _object_key: &str,
        _upload_id: &str,
    ) -> Result<(), StorageError> {
        Err(StorageError::Unexpected)
    }

    async fn delete_object(&self, _object_key: &str) -> Result<(), StorageError> {
        Err(StorageError::Unexpected)
    }

    async fn list_objects(&self) -> Result<Vec<StoredObject>, StorageError> {
        Err(StorageError::Unexpected)
    }
}
