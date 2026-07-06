use std::sync::Arc;

use crate::application::AppError;
use crate::application::ports::files::FileRepository;
use crate::domain::auth::User;
use crate::domain::files::DriveBrowse;

#[derive(Clone)]
pub struct ListDriveTrashUseCase {
    file_repository: Arc<dyn FileRepository>,
}

impl ListDriveTrashUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>) -> Self {
        Self { file_repository }
    }

    pub async fn execute(&self, user: &User) -> Result<DriveBrowse, AppError> {
        self.file_repository
            .list_drive_trash(user.id)
            .await
            .map_err(AppError::from)
    }
}
