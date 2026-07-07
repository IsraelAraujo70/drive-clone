use std::sync::Arc;

use crate::application::AppError;
use crate::application::ports::auth::AuthRepository;
use crate::application::ports::clock::Clock;
use crate::domain::auth::{User, hash_token};

#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub user: User,
    pub token_hash: String,
}

#[derive(Clone)]
pub struct GetCurrentUserUseCase {
    auth_repository: Arc<dyn AuthRepository>,
    clock: Arc<dyn Clock>,
}

impl GetCurrentUserUseCase {
    pub fn new(auth_repository: Arc<dyn AuthRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            auth_repository,
            clock,
        }
    }

    pub async fn execute(&self, token: &str) -> Result<CurrentUser, AppError> {
        let token_hash = hash_token(token);
        let user = self
            .auth_repository
            .find_user_by_session_hash(&token_hash, self.clock.now())
            .await?
            .ok_or(AppError::Unauthorized)?;
        Ok(CurrentUser { user, token_hash })
    }
}
