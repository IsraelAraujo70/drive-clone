use async_trait::async_trait;
use uuid::Uuid;

use crate::application::ports::RepositoryError;
use crate::domain::files::{DriveBrowse, DriveFile, FileShare, Folder, PendingFile, SharedFile};

#[derive(Debug, Clone)]
pub struct CreatePendingFileRecord {
    pub owner_id: Uuid,
    pub filename: String,
    pub parent_folder_id: Option<Uuid>,
    pub content_type: String,
    pub size_bytes: i64,
    pub checksum_sha256: Option<String>,
    pub object_key: String,
}

#[derive(Debug, Clone)]
pub struct CreateFolderRecord {
    pub owner_id: Uuid,
    pub name: String,
    pub parent_folder_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct UpdateFileRecord {
    pub owner_id: Uuid,
    pub file_id: Uuid,
    pub filename: Option<String>,
    pub parent_folder_id: Option<Option<Uuid>>,
}

#[derive(Debug, Clone)]
pub struct UpdateFolderRecord {
    pub owner_id: Uuid,
    pub folder_id: Uuid,
    pub name: Option<String>,
    pub parent_folder_id: Option<Option<Uuid>>,
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

    async fn create_folder(&self, input: CreateFolderRecord) -> Result<Folder, RepositoryError>;

    async fn browse_folder(
        &self,
        owner_id: Uuid,
        parent_folder_id: Option<Uuid>,
    ) -> Result<DriveBrowse, RepositoryError>;

    async fn list_active_folders(&self, owner_id: Uuid) -> Result<Vec<Folder>, RepositoryError>;

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

    async fn update_owned_file(
        &self,
        input: UpdateFileRecord,
    ) -> Result<DriveFile, RepositoryError>;

    async fn update_owned_folder(
        &self,
        input: UpdateFolderRecord,
    ) -> Result<Folder, RepositoryError>;

    async fn find_downloadable_file(
        &self,
        user_id: Uuid,
        file_id: Uuid,
    ) -> Result<Option<DriveFile>, RepositoryError>;

    async fn soft_delete_owned_file(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<(), RepositoryError>;

    async fn restore_owned_file(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<DriveFile, RepositoryError>;

    async fn list_trash(&self, owner_id: Uuid) -> Result<Vec<DriveFile>, RepositoryError>;

    async fn soft_delete_owned_folder_tree(
        &self,
        owner_id: Uuid,
        folder_id: Uuid,
    ) -> Result<(), RepositoryError>;

    async fn restore_owned_folder_tree(
        &self,
        owner_id: Uuid,
        folder_id: Uuid,
    ) -> Result<Folder, RepositoryError>;

    async fn list_drive_trash(&self, owner_id: Uuid) -> Result<DriveBrowse, RepositoryError>;

    async fn create_share(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
        grantee_id: Uuid,
    ) -> Result<FileShare, RepositoryError>;

    async fn list_shares(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<Vec<FileShare>, RepositoryError>;

    async fn revoke_share(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
        grantee_id: Uuid,
    ) -> Result<(), RepositoryError>;

    async fn list_shared_with_me(
        &self,
        grantee_id: Uuid,
    ) -> Result<Vec<SharedFile>, RepositoryError>;
}
