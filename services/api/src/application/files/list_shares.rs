use std::sync::Arc;

use uuid::Uuid;

use crate::application::AppError;
use crate::application::ports::files::FileRepository;
use crate::domain::auth::User;
use crate::domain::files::FileShare;

#[derive(Clone)]
pub struct ListSharesUseCase {
    file_repository: Arc<dyn FileRepository>,
}

impl ListSharesUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>) -> Self {
        Self { file_repository }
    }

    pub async fn execute(&self, user: &User, file_id: Uuid) -> Result<Vec<FileShare>, AppError> {
        self.file_repository
            .list_shares(user.id, file_id)
            .await
            .map_err(AppError::from)
    }
}
