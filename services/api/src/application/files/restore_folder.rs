use std::sync::Arc;

use uuid::Uuid;

use crate::application::AppError;
use crate::application::ports::files::FileRepository;
use crate::domain::auth::User;
use crate::domain::files::Folder;

#[derive(Clone)]
pub struct RestoreFolderUseCase {
    file_repository: Arc<dyn FileRepository>,
}

impl RestoreFolderUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>) -> Self {
        Self { file_repository }
    }

    pub async fn execute(&self, user: &User, folder_id: Uuid) -> Result<Folder, AppError> {
        self.file_repository
            .restore_owned_folder_tree(user.id, folder_id)
            .await
            .map_err(AppError::from)
    }
}
