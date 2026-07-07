use std::sync::Arc;

use uuid::Uuid;

use crate::application::AppError;
use crate::application::ports::files::{FileRepository, UpdateFolderRecord};
use crate::domain::auth::User;
use crate::domain::error::DomainError;
use crate::domain::files::{Folder, validate_folder_name};

#[derive(Debug, Clone)]
pub struct UpdateFolderInput {
    pub folder_id: Uuid,
    pub name: Option<String>,
    pub parent_folder_id: Option<Option<Uuid>>,
}

#[derive(Clone)]
pub struct UpdateFolderUseCase {
    file_repository: Arc<dyn FileRepository>,
}

impl UpdateFolderUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>) -> Self {
        Self { file_repository }
    }

    pub async fn execute(&self, user: &User, input: UpdateFolderInput) -> Result<Folder, AppError> {
        if input.name.is_none() && input.parent_folder_id.is_none() {
            return Err(DomainError::Validation("Provide a folder name or parent folder").into());
        }
        let name = input
            .name
            .as_deref()
            .map(validate_folder_name)
            .transpose()?;

        self.file_repository
            .update_owned_folder(UpdateFolderRecord {
                owner_id: user.id,
                folder_id: input.folder_id,
                name,
                parent_folder_id: input.parent_folder_id,
            })
            .await
            .map_err(AppError::from)
    }
}
