use chrono::{DateTime, Utc};
use serde::de::Deserializer;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::application::auth::signup::AuthResponse as UseCaseAuthResponse;
use crate::application::files::{
    CreateFolderInput, CreateResumableUploadOutput, CreateUploadOutput, DownloadFileOutput,
    ExpireUploadsOutput, PresignUploadPartInput, PresignUploadPartOutput, RecordUploadPartInput,
    SearchFilesInput, ShareFileInput, UpdateFileInput, UpdateFolderInput,
};
use crate::domain::auth::User;
use crate::domain::files::{
    DriveBrowse, DriveFile, FileShare, FileUser, Folder, FolderPathEntry, PendingUpload,
    ResumableUploadRequest, ResumableUploadSession, SearchFileResult, SharedFile, UploadPart,
    UploadRequest,
};

#[derive(Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
    pub display_name: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    user: User,
    token: String,
}

impl From<UseCaseAuthResponse> for AuthResponse {
    fn from(response: UseCaseAuthResponse) -> Self {
        Self {
            user: response.user,
            token: response.token,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateUploadRequest {
    filename: String,
    parent_folder_id: Option<Uuid>,
    content_type: String,
    size_bytes: i64,
    checksum_sha256: Option<String>,
}

impl From<CreateUploadRequest> for UploadRequest {
    fn from(request: CreateUploadRequest) -> Self {
        Self {
            filename: request.filename,
            parent_folder_id: request.parent_folder_id,
            content_type: request.content_type,
            size_bytes: request.size_bytes,
            checksum_sha256: request.checksum_sha256,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CreateUploadResponse {
    file_id: Uuid,
    upload_url: String,
    object_key: String,
    expires_at: DateTime<Utc>,
}

impl From<CreateUploadOutput> for CreateUploadResponse {
    fn from(output: CreateUploadOutput) -> Self {
        Self {
            file_id: output.file_id,
            upload_url: output.upload_url,
            object_key: output.object_key,
            expires_at: output.expires_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateResumableUploadRequest {
    filename: String,
    parent_folder_id: Option<Uuid>,
    content_type: String,
    size_bytes: i64,
    checksum_sha256: Option<String>,
    part_size_bytes: Option<i64>,
}

impl From<CreateResumableUploadRequest> for ResumableUploadRequest {
    fn from(request: CreateResumableUploadRequest) -> Self {
        Self {
            filename: request.filename,
            parent_folder_id: request.parent_folder_id,
            content_type: request.content_type,
            size_bytes: request.size_bytes,
            checksum_sha256: request.checksum_sha256,
            part_size_bytes: request.part_size_bytes,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CreateResumableUploadResponse {
    file_id: Uuid,
    object_key: String,
    part_size_bytes: i64,
    expires_at: DateTime<Utc>,
}

impl From<CreateResumableUploadOutput> for CreateResumableUploadResponse {
    fn from(output: CreateResumableUploadOutput) -> Self {
        Self {
            file_id: output.file_id,
            object_key: output.object_key,
            part_size_bytes: output.part_size_bytes,
            expires_at: output.expires_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UploadPartResponse {
    part_number: i32,
    size_bytes: i64,
    etag: String,
}

impl From<UploadPart> for UploadPartResponse {
    fn from(part: UploadPart) -> Self {
        Self {
            part_number: part.part_number,
            size_bytes: part.size_bytes,
            etag: part.etag,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UploadStatusResponse {
    file_id: Uuid,
    filename: String,
    parent_folder_id: Option<Uuid>,
    content_type: String,
    size_bytes: i64,
    checksum_sha256: Option<String>,
    object_key: String,
    state: String,
    part_size_bytes: i64,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    parts: Vec<UploadPartResponse>,
}

impl From<ResumableUploadSession> for UploadStatusResponse {
    fn from(session: ResumableUploadSession) -> Self {
        Self {
            file_id: session.file_id,
            filename: session.filename,
            parent_folder_id: session.parent_folder_id,
            content_type: session.content_type,
            size_bytes: session.size_bytes,
            checksum_sha256: session.checksum_sha256,
            object_key: session.object_key,
            state: session.state.as_str().to_string(),
            part_size_bytes: session.part_size_bytes,
            expires_at: session.upload_expires_at,
            created_at: session.created_at,
            updated_at: session.updated_at,
            completed_at: session.completed_at,
            parts: session.parts.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PendingUploadResponse {
    file_id: Uuid,
    filename: String,
    parent_folder_id: Option<Uuid>,
    size_bytes: i64,
    part_size_bytes: i64,
    checksum_sha256: Option<String>,
    parts_received: i64,
    expires_at: DateTime<Utc>,
}

impl From<PendingUpload> for PendingUploadResponse {
    fn from(upload: PendingUpload) -> Self {
        Self {
            file_id: upload.file_id,
            filename: upload.filename,
            parent_folder_id: upload.parent_folder_id,
            size_bytes: upload.size_bytes,
            part_size_bytes: upload.part_size_bytes,
            checksum_sha256: upload.checksum_sha256,
            parts_received: upload.parts_received,
            expires_at: upload.expires_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListPendingUploadsResponse {
    uploads: Vec<PendingUploadResponse>,
}

impl From<Vec<PendingUpload>> for ListPendingUploadsResponse {
    fn from(uploads: Vec<PendingUpload>) -> Self {
        Self {
            uploads: uploads.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PresignUploadPartRequest {
    part_number: i32,
}

impl PresignUploadPartRequest {
    pub fn into_input(self, file_id: Uuid) -> PresignUploadPartInput {
        PresignUploadPartInput {
            file_id,
            part_number: self.part_number,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PresignUploadPartResponse {
    file_id: Uuid,
    part_number: i32,
    upload_url: String,
    expires_at: DateTime<Utc>,
    expected_size_bytes: i64,
}

impl From<PresignUploadPartOutput> for PresignUploadPartResponse {
    fn from(output: PresignUploadPartOutput) -> Self {
        Self {
            file_id: output.file_id,
            part_number: output.part_number,
            upload_url: output.upload_url,
            expires_at: output.expires_at,
            expected_size_bytes: output.expected_size_bytes,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RecordUploadPartRequest {
    size_bytes: i64,
    etag: String,
}

impl RecordUploadPartRequest {
    pub fn into_input(self, file_id: Uuid, part_number: i32) -> RecordUploadPartInput {
        RecordUploadPartInput {
            file_id,
            part_number,
            size_bytes: self.size_bytes,
            etag: self.etag,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ExpireUploadsResponse {
    expired_count: usize,
    aborted_count: usize,
}

impl From<ExpireUploadsOutput> for ExpireUploadsResponse {
    fn from(output: ExpireUploadsOutput) -> Self {
        Self {
            expired_count: output.expired_count,
            aborted_count: output.aborted_count,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FileResponse {
    id: Uuid,
    filename: String,
    parent_folder_id: Option<Uuid>,
    content_type: String,
    size_bytes: i64,
    checksum_sha256: Option<String>,
    object_key: String,
    state: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
}

impl From<DriveFile> for FileResponse {
    fn from(file: DriveFile) -> Self {
        Self {
            id: file.id,
            filename: file.filename,
            parent_folder_id: file.parent_folder_id,
            content_type: file.content_type,
            size_bytes: file.size_bytes,
            checksum_sha256: file.checksum_sha256,
            object_key: file.object_key,
            state: file.state.as_str().to_string(),
            created_at: file.created_at,
            updated_at: file.updated_at,
            completed_at: file.completed_at,
            deleted_at: file.deleted_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateFolderRequest {
    name: String,
    parent_folder_id: Option<Uuid>,
}

impl From<CreateFolderRequest> for CreateFolderInput {
    fn from(request: CreateFolderRequest) -> Self {
        Self {
            name: request.name,
            parent_folder_id: request.parent_folder_id,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FolderResponse {
    id: Uuid,
    name: String,
    parent_folder_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}

impl From<Folder> for FolderResponse {
    fn from(folder: Folder) -> Self {
        Self {
            id: folder.id,
            name: folder.name,
            parent_folder_id: folder.parent_folder_id,
            created_at: folder.created_at,
            updated_at: folder.updated_at,
            deleted_at: folder.deleted_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FolderPathEntryResponse {
    id: Uuid,
    name: String,
}

impl From<FolderPathEntry> for FolderPathEntryResponse {
    fn from(entry: FolderPathEntry) -> Self {
        Self {
            id: entry.id,
            name: entry.name,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DriveBrowseResponse {
    parent_folder_id: Option<Uuid>,
    breadcrumbs: Vec<FolderPathEntryResponse>,
    folders: Vec<FolderResponse>,
    files: Vec<FileResponse>,
}

impl From<DriveBrowse> for DriveBrowseResponse {
    fn from(browse: DriveBrowse) -> Self {
        Self {
            parent_folder_id: browse.parent_folder_id,
            breadcrumbs: browse.breadcrumbs.into_iter().map(Into::into).collect(),
            folders: browse.folders.into_iter().map(Into::into).collect(),
            files: browse.files.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListFoldersResponse {
    folders: Vec<FolderResponse>,
}

impl From<Vec<Folder>> for ListFoldersResponse {
    fn from(folders: Vec<Folder>) -> Self {
        Self {
            folders: folders.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct BrowseDriveQuery {
    pub parent_folder_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct SearchFilesQuery {
    pub q: String,
    #[serde(default)]
    pub include_deleted: bool,
    pub limit: Option<i64>,
}

impl SearchFilesQuery {
    pub fn normalized_query(&self) -> String {
        self.q.trim().to_string()
    }
}

impl From<SearchFilesQuery> for SearchFilesInput {
    fn from(query: SearchFilesQuery) -> Self {
        Self {
            query: query.q,
            include_deleted: query.include_deleted,
            limit: query.limit,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateFileRequest {
    filename: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_uuid_patch_field")]
    parent_folder_id: Option<Option<Uuid>>,
}

impl UpdateFileRequest {
    pub fn into_input(self, file_id: Uuid) -> UpdateFileInput {
        UpdateFileInput {
            file_id,
            filename: self.filename,
            parent_folder_id: self.parent_folder_id,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateFolderRequest {
    name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_uuid_patch_field")]
    parent_folder_id: Option<Option<Uuid>>,
}

impl UpdateFolderRequest {
    pub fn into_input(self, folder_id: Uuid) -> UpdateFolderInput {
        UpdateFolderInput {
            folder_id,
            name: self.name,
            parent_folder_id: self.parent_folder_id,
        }
    }
}

fn deserialize_nullable_uuid_patch_field<'de, D>(
    deserializer: D,
) -> Result<Option<Option<Uuid>>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<Uuid>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Serialize)]
pub struct ListFilesResponse {
    files: Vec<FileResponse>,
}

impl From<Vec<DriveFile>> for ListFilesResponse {
    fn from(files: Vec<DriveFile>) -> Self {
        Self {
            files: files.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DownloadResponse {
    download_url: String,
    expires_at: DateTime<Utc>,
}

impl From<DownloadFileOutput> for DownloadResponse {
    fn from(output: DownloadFileOutput) -> Self {
        Self {
            download_url: output.download_url,
            expires_at: output.expires_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ShareFileRequest {
    email: String,
}

impl ShareFileRequest {
    pub fn into_input(self, file_id: Uuid) -> ShareFileInput {
        ShareFileInput {
            file_id,
            email: self.email,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FileUserResponse {
    id: Uuid,
    email: String,
    display_name: String,
}

impl From<FileUser> for FileUserResponse {
    fn from(user: FileUser) -> Self {
        Self {
            id: user.id,
            email: user.email,
            display_name: user.display_name,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FileShareResponse {
    file_id: Uuid,
    grantee: FileUserResponse,
    created_at: DateTime<Utc>,
}

impl From<FileShare> for FileShareResponse {
    fn from(share: FileShare) -> Self {
        Self {
            file_id: share.file_id,
            grantee: share.grantee.into(),
            created_at: share.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListSharesResponse {
    shares: Vec<FileShareResponse>,
}

impl From<Vec<FileShare>> for ListSharesResponse {
    fn from(shares: Vec<FileShare>) -> Self {
        Self {
            shares: shares.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SharedFileResponse {
    #[serde(flatten)]
    file: FileResponse,
    owner: FileUserResponse,
}

impl From<SharedFile> for SharedFileResponse {
    fn from(shared: SharedFile) -> Self {
        Self {
            file: shared.file.into(),
            owner: shared.owner.into(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListSharedWithMeResponse {
    files: Vec<SharedFileResponse>,
}

impl From<Vec<SharedFile>> for ListSharedWithMeResponse {
    fn from(files: Vec<SharedFile>) -> Self {
        Self {
            files: files.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SearchFileResultResponse {
    access: String,
    file: FileResponse,
    owner: Option<FileUserResponse>,
}

impl From<SearchFileResult> for SearchFileResultResponse {
    fn from(result: SearchFileResult) -> Self {
        Self {
            access: result.access.as_str().to_string(),
            file: result.file.into(),
            owner: result.owner.map(Into::into),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SearchFilesResponse {
    query: String,
    files: Vec<SearchFileResultResponse>,
}

impl SearchFilesResponse {
    pub fn new(query: String, files: Vec<SearchFileResult>) -> Self {
        Self {
            query,
            files: files.into_iter().map(Into::into).collect(),
        }
    }
}
