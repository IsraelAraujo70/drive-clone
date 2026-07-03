use std::sync::Arc;

use uuid::Uuid;

use crate::application::AppError;
use crate::application::ports::files::FileRepository;
use crate::domain::auth::User;

#[derive(Clone)]
pub struct DeleteFileUseCase {
    file_repository: Arc<dyn FileRepository>,
}

impl DeleteFileUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>) -> Self {
        Self { file_repository }
    }

    pub async fn execute(&self, user: &User, file_id: Uuid) -> Result<(), AppError> {
        self.file_repository
            .soft_delete_owned_file(user.id, file_id)
            .await
            .map_err(AppError::from)
    }
}
