use async_trait::async_trait;
use uuid::Uuid;

use crate::application::ports::RepositoryError;
use crate::domain::files::{DriveFile, PendingFile};

#[derive(Debug, Clone)]
pub struct CreatePendingFileRecord {
    pub owner_id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub checksum_sha256: Option<String>,
    pub object_key: String,
}

#[async_trait]
pub trait FileRepository: Send + Sync {
    async fn pending_bytes_for_owner(
        &self,
        owner_id: Uuid,
        cap: i64,
    ) -> Result<i64, RepositoryError>;

    async fn create_pending_file(
        &self,
        input: CreatePendingFileRecord,
    ) -> Result<PendingFile, RepositoryError>;

    async fn complete_upload_once(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
        expected_size: i64,
    ) -> Result<DriveFile, RepositoryError>;

    async fn find_owned_file_for_completion(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<Option<PendingFile>, RepositoryError>;

    async fn list_completed_files(&self, owner_id: Uuid)
    -> Result<Vec<DriveFile>, RepositoryError>;

    async fn find_completed_owned_file(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<Option<DriveFile>, RepositoryError>;
}
