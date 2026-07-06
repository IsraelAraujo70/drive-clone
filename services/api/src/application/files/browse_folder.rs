use std::sync::Arc;

use uuid::Uuid;

use crate::application::AppError;
use crate::application::ports::files::FileRepository;
use crate::domain::auth::User;
use crate::domain::files::DriveBrowse;

#[derive(Clone)]
pub struct BrowseFolderUseCase {
    file_repository: Arc<dyn FileRepository>,
}

impl BrowseFolderUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>) -> Self {
        Self { file_repository }
    }

    pub async fn execute(
        &self,
        user: &User,
        parent_folder_id: Option<Uuid>,
    ) -> Result<DriveBrowse, AppError> {
        self.file_repository
            .browse_folder(user.id, parent_folder_id)
            .await
            .map_err(AppError::from)
    }
}
