use std::sync::Arc;

use crate::application::AppError;
use crate::application::ports::files::FileRepository;
use crate::domain::auth::User;
use crate::domain::files::DriveFile;

#[derive(Clone)]
pub struct ListTrashUseCase {
    file_repository: Arc<dyn FileRepository>,
}

impl ListTrashUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>) -> Self {
        Self { file_repository }
    }

    pub async fn execute(&self, user: &User) -> Result<Vec<DriveFile>, AppError> {
        self.file_repository
            .list_trash(user.id)
            .await
            .map_err(AppError::from)
    }
}
