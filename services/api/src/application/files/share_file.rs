use std::sync::Arc;

use uuid::Uuid;

use crate::application::AppError;
use crate::application::ports::auth::AuthRepository;
use crate::application::ports::files::FileRepository;
use crate::domain::auth::{User, validate_email};
use crate::domain::error::DomainError;
use crate::domain::files::FileShare;

#[derive(Debug, Clone)]
pub struct ShareFileInput {
    pub file_id: Uuid,
    pub email: String,
}

#[derive(Clone)]
pub struct ShareFileUseCase {
    file_repository: Arc<dyn FileRepository>,
    auth_repository: Arc<dyn AuthRepository>,
}

impl ShareFileUseCase {
    pub fn new(
        file_repository: Arc<dyn FileRepository>,
        auth_repository: Arc<dyn AuthRepository>,
    ) -> Self {
        Self {
            file_repository,
            auth_repository,
        }
    }

    pub async fn execute(
        &self,
        owner: &User,
        input: ShareFileInput,
    ) -> Result<FileShare, AppError> {
        let email = input.email.trim().to_lowercase();
        validate_email(&email)?;

        let grantee = self
            .auth_repository
            .find_user_by_email(&email)
            .await?
            .ok_or(DomainError::UserNotFound)?;

        if grantee.id == owner.id {
            return Err(DomainError::Validation("Cannot share a file with yourself").into());
        }

        self.file_repository
            .create_share(owner.id, input.file_id, grantee.id)
            .await
            .map_err(AppError::from)
    }
}
