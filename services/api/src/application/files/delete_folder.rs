use std::sync::Arc;

use uuid::Uuid;

use crate::application::AppError;
use crate::application::ports::files::FileRepository;
use crate::domain::auth::User;

#[derive(Clone)]
pub struct DeleteFolderUseCase {
    file_repository: Arc<dyn FileRepository>,
}

impl DeleteFolderUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>) -> Self {
        Self { file_repository }
    }

    pub async fn execute(&self, user: &User, folder_id: Uuid) -> Result<(), AppError> {
        self.file_repository
            .soft_delete_owned_folder_tree(user.id, folder_id)
            .await
            .map_err(AppError::from)
    }
}
