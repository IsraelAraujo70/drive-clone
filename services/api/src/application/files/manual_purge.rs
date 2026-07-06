use std::sync::Arc;

use uuid::Uuid;

use crate::application::AppError;
use crate::application::ports::StorageError;
use crate::application::ports::files::FileRepository;
use crate::application::ports::object_storage::ObjectStorage;
use crate::domain::auth::User;
use crate::domain::error::DomainError;

#[derive(Clone)]
pub struct PurgeFileUseCase {
    file_repository: Arc<dyn FileRepository>,
    storage: Arc<dyn ObjectStorage>,
}

impl PurgeFileUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>, storage: Arc<dyn ObjectStorage>) -> Self {
        Self {
            file_repository,
            storage,
        }
    }

    pub async fn execute(&self, user: &User, file_id: Uuid) -> Result<(), AppError> {
        let target = self
            .file_repository
            .find_manual_purge_file_target(user.id, file_id)
            .await?
            .ok_or(AppError::Domain(DomainError::FileNotFound))?;

        match self.storage.delete_object(&target.object_key).await {
            Ok(()) | Err(StorageError::NotFound) => {}
            Err(error) => return Err(AppError::from(error)),
        }

        self.file_repository
            .manual_purge_file(user.id, target.file_id)
            .await?
            .ok_or(AppError::Domain(DomainError::FileNotFound))?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct PurgeFolderUseCase {
    file_repository: Arc<dyn FileRepository>,
    storage: Arc<dyn ObjectStorage>,
}

impl PurgeFolderUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>, storage: Arc<dyn ObjectStorage>) -> Self {
        Self {
            file_repository,
            storage,
        }
    }

    pub async fn execute(&self, user: &User, folder_id: Uuid) -> Result<(), AppError> {
        let targets = self
            .file_repository
            .find_manual_purge_folder_targets(user.id, folder_id)
            .await?
            .ok_or(AppError::Domain(DomainError::FileNotFound))?;

        for target in targets {
            match self.storage.delete_object(&target.object_key).await {
                Ok(()) | Err(StorageError::NotFound) => {}
                Err(error) => return Err(AppError::from(error)),
            }
        }

        if !self
            .file_repository
            .manual_purge_folder_tree(user.id, folder_id)
            .await?
        {
            return Err(AppError::Domain(DomainError::FileNotFound));
        }

        Ok(())
    }
}
