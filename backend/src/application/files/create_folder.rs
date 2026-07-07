use std::sync::Arc;

use uuid::Uuid;

use crate::application::AppError;
use crate::application::ports::files::{CreateFolderRecord, FileRepository};
use crate::domain::auth::User;
use crate::domain::files::{Folder, validate_folder_name};

#[derive(Debug, Clone)]
pub struct CreateFolderInput {
    pub name: String,
    pub parent_folder_id: Option<Uuid>,
}

#[derive(Clone)]
pub struct CreateFolderUseCase {
    file_repository: Arc<dyn FileRepository>,
}

impl CreateFolderUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>) -> Self {
        Self { file_repository }
    }

    pub async fn execute(&self, user: &User, input: CreateFolderInput) -> Result<Folder, AppError> {
        let name = validate_folder_name(&input.name)?;
        self.file_repository
            .create_folder(CreateFolderRecord {
                owner_id: user.id,
                name,
                parent_folder_id: input.parent_folder_id,
            })
            .await
            .map_err(AppError::from)
    }
}
