use std::sync::Arc;

use crate::application::AppError;
use crate::application::ports::auth::AuthRepository;
use crate::domain::auth::hash_token;

#[derive(Clone)]
pub struct LogoutUseCase {
    auth_repository: Arc<dyn AuthRepository>,
}

impl LogoutUseCase {
    pub fn new(auth_repository: Arc<dyn AuthRepository>) -> Self {
        Self { auth_repository }
    }

    pub async fn execute(&self, token: &str) -> Result<(), AppError> {
        self.execute_hash(&hash_token(token)).await
    }

    pub async fn execute_hash(&self, token_hash: &str) -> Result<(), AppError> {
        self.auth_repository
            .delete_session(token_hash)
            .await
            .map_err(AppError::from)
    }
}
