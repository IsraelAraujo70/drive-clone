use async_trait::async_trait;
use uuid::Uuid;

use crate::application::ports::RepositoryError;
use crate::domain::files::{
    DriveBrowse, DriveFile, FileShare, Folder, PendingFile, PendingUpload, ResumableUploadSession,
    SearchFileResult, SharedFile, UploadPart,
};

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
pub struct CreateResumableUploadRecord {
    pub owner_id: Uuid,
    pub filename: String,
    pub parent_folder_id: Option<Uuid>,
    pub content_type: String,
    pub size_bytes: i64,
    pub checksum_sha256: Option<String>,
    pub object_key: String,
    pub multipart_upload_id: String,
    pub part_size_bytes: i64,
    pub upload_expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct RecordUploadPartRecord {
    pub owner_id: Uuid,
    pub file_id: Uuid,
    pub part_number: i32,
    pub size_bytes: i64,
    pub etag: String,
}

#[derive(Debug, Clone)]
pub struct ExpiredUploadRecord {
    pub file_id: Uuid,
    pub object_key: String,
    pub multipart_upload_id: String,
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

#[derive(Debug, Clone)]
pub struct SearchFilesRecord {
    pub user_id: Uuid,
    pub query: String,
    pub include_deleted: bool,
    pub limit: i64,
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

    async fn create_resumable_upload(
        &self,
        input: CreateResumableUploadRecord,
    ) -> Result<ResumableUploadSession, RepositoryError>;

    async fn find_resumable_upload(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<Option<ResumableUploadSession>, RepositoryError>;

    async fn list_pending_resumable_uploads(
        &self,
        owner_id: Uuid,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<PendingUpload>, RepositoryError>;

    async fn record_upload_part(
        &self,
        input: RecordUploadPartRecord,
    ) -> Result<UploadPart, RepositoryError>;

    async fn complete_resumable_upload_once(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
        expected_size: i64,
    ) -> Result<DriveFile, RepositoryError>;

    async fn expire_resumable_uploads(
        &self,
        now: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<Vec<ExpiredUploadRecord>, RepositoryError>;

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

    async fn search_accessible_files(
        &self,
        input: SearchFilesRecord,
    ) -> Result<Vec<SearchFileResult>, RepositoryError>;
}
