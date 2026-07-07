use std::sync::Arc;

use uuid::Uuid;

use crate::application::AppError;
use crate::application::ports::files::{FileRepository, UpdateFileRecord};
use crate::domain::auth::User;
use crate::domain::error::DomainError;
use crate::domain::files::{DriveFile, validate_filename};

#[derive(Debug, Clone)]
pub struct UpdateFileInput {
    pub file_id: Uuid,
    pub filename: Option<String>,
    pub parent_folder_id: Option<Option<Uuid>>,
}

#[derive(Clone)]
pub struct UpdateFileUseCase {
    file_repository: Arc<dyn FileRepository>,
}

impl UpdateFileUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>) -> Self {
        Self { file_repository }
    }

    pub async fn execute(
        &self,
        user: &User,
        input: UpdateFileInput,
    ) -> Result<DriveFile, AppError> {
        if input.filename.is_none() && input.parent_folder_id.is_none() {
            return Err(DomainError::Validation("Provide a filename or parent folder").into());
        }
        let filename = input
            .filename
            .as_deref()
            .map(validate_filename)
            .transpose()?;

        self.file_repository
            .update_owned_file(UpdateFileRecord {
                owner_id: user.id,
                file_id: input.file_id,
                filename,
                parent_folder_id: input.parent_folder_id,
            })
            .await
            .map_err(AppError::from)
    }
}
