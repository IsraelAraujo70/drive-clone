use std::sync::Arc;

use uuid::Uuid;

use crate::application::AppError;
use crate::application::ports::files::FileRepository;
use crate::domain::auth::User;
use crate::domain::files::DriveFile;

#[derive(Clone)]
pub struct RestoreFileUseCase {
    file_repository: Arc<dyn FileRepository>,
}

impl RestoreFileUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>) -> Self {
        Self { file_repository }
    }

    pub async fn execute(&self, user: &User, file_id: Uuid) -> Result<DriveFile, AppError> {
        self.file_repository
            .restore_owned_file(user.id, file_id)
            .await
            .map_err(AppError::from)
    }
}
