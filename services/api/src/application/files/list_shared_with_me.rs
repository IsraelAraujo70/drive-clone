use std::sync::Arc;

use crate::application::AppError;
use crate::application::ports::files::FileRepository;
use crate::domain::auth::User;
use crate::domain::files::SharedFile;

#[derive(Clone)]
pub struct ListSharedWithMeUseCase {
    file_repository: Arc<dyn FileRepository>,
}

impl ListSharedWithMeUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>) -> Self {
        Self { file_repository }
    }

    pub async fn execute(&self, user: &User) -> Result<Vec<SharedFile>, AppError> {
        self.file_repository
            .list_shared_with_me(user.id)
            .await
            .map_err(AppError::from)
    }
}
