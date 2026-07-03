use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::application::auth::signup::AuthResponse as UseCaseAuthResponse;
use crate::application::files::{CreateUploadOutput, DownloadFileOutput};
use crate::domain::auth::User;
use crate::domain::files::{DriveFile, UploadRequest};

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
    content_type: String,
    size_bytes: i64,
    checksum_sha256: Option<String>,
}

impl From<CreateUploadRequest> for UploadRequest {
    fn from(request: CreateUploadRequest) -> Self {
        Self {
            filename: request.filename,
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

#[derive(Debug, Serialize)]
pub struct FileResponse {
    id: Uuid,
    filename: String,
    content_type: String,
    size_bytes: i64,
    checksum_sha256: Option<String>,
    object_key: String,
    state: String,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

impl From<DriveFile> for FileResponse {
    fn from(file: DriveFile) -> Self {
        Self {
            id: file.id,
            filename: file.filename,
            content_type: file.content_type,
            size_bytes: file.size_bytes,
            checksum_sha256: file.checksum_sha256,
            object_key: file.object_key,
            state: file.state.as_str().to_string(),
            created_at: file.created_at,
            completed_at: file.completed_at,
        }
    }
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
