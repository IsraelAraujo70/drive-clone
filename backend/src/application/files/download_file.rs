use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::AppError;
use crate::application::ports::files::FileRepository;
use crate::application::ports::object_storage::ObjectStorage;
use crate::domain::auth::User;
use crate::domain::error::DomainError;

#[derive(Debug, Clone)]
pub struct DownloadFileOutput {
    pub download_url: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct DownloadFileUseCase {
    file_repository: Arc<dyn FileRepository>,
    storage: Arc<dyn ObjectStorage>,
    presigned_url_ttl_seconds: i64,
}

impl DownloadFileUseCase {
    pub fn new(
        file_repository: Arc<dyn FileRepository>,
        storage: Arc<dyn ObjectStorage>,
        presigned_url_ttl_seconds: i64,
    ) -> Self {
        Self {
            file_repository,
            storage,
            presigned_url_ttl_seconds,
        }
    }

    pub async fn execute(
        &self,
        user: &User,
        file_id: Uuid,
    ) -> Result<DownloadFileOutput, AppError> {
        let file = self
            .file_repository
            .find_downloadable_file(user.id, file_id)
            .await?
            .ok_or(DomainError::FileNotFound)?;

        let presigned = self
            .storage
            .presign_get(&file.object_key, self.presigned_url_ttl_seconds)
            .await?;

        Ok(DownloadFileOutput {
            download_url: presigned.url,
            expires_at: presigned.expires_at,
        })
    }
}
