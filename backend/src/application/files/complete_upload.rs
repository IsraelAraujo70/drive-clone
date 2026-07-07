use std::sync::Arc;

use uuid::Uuid;

use crate::application::AppError;
use crate::application::ports::files::FileRepository;
use crate::application::ports::object_storage::ObjectStorage;
use crate::domain::auth::User;
use crate::domain::error::DomainError;
use crate::domain::files::{DriveFile, FileState};

#[derive(Clone)]
pub struct CompleteUploadUseCase {
    file_repository: Arc<dyn FileRepository>,
    storage: Arc<dyn ObjectStorage>,
}

impl CompleteUploadUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>, storage: Arc<dyn ObjectStorage>) -> Self {
        Self {
            file_repository,
            storage,
        }
    }

    pub async fn execute(&self, user: &User, file_id: Uuid) -> Result<DriveFile, AppError> {
        let row = self
            .file_repository
            .find_owned_file_for_completion(user.id, file_id)
            .await?
            .ok_or(DomainError::FileNotFound)?;

        if row.state != FileState::Pending {
            return Err(DomainError::InvalidFileState.into());
        }

        let object = self.storage.head_object(&row.object_key).await?;
        if object.content_length != row.size_bytes {
            return Err(AppError::Storage);
        }

        self.file_repository
            .complete_upload_once(user.id, file_id, row.size_bytes)
            .await
            .map_err(AppError::from)
    }
}
