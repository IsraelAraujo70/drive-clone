use std::sync::Arc;

use uuid::Uuid;

use crate::application::AppError;
use crate::application::ports::files::FileRepository;
use crate::domain::auth::User;
use crate::domain::files::ShareLink;

#[derive(Clone)]
pub struct ListShareLinksUseCase {
    file_repository: Arc<dyn FileRepository>,
}

impl ListShareLinksUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>) -> Self {
        Self { file_repository }
    }

    pub async fn execute(&self, owner: &User, file_id: Uuid) -> Result<Vec<ShareLink>, AppError> {
        self.file_repository
            .list_share_links(owner.id, file_id)
            .await
            .map_err(AppError::from)
    }
}
