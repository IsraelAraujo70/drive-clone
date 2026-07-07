use std::sync::Arc;

use crate::application::AppError;
use crate::application::ports::files::FileRepository;
use crate::domain::auth::User;
use crate::domain::files::Folder;

#[derive(Clone)]
pub struct ListFoldersUseCase {
    file_repository: Arc<dyn FileRepository>,
}

impl ListFoldersUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>) -> Self {
        Self { file_repository }
    }

    pub async fn execute(&self, user: &User) -> Result<Vec<Folder>, AppError> {
        self.file_repository
            .list_active_folders(user.id)
            .await
            .map_err(AppError::from)
    }
}
