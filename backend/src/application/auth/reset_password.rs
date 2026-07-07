use std::sync::Arc;

use crate::application::AppError;
use crate::application::ports::auth::AuthRepository;
use crate::application::ports::clock::Clock;
use crate::domain::auth::{hash_password, hash_token, validate_password};
use crate::domain::error::DomainError;

#[derive(Debug, Clone)]
pub struct ResetPasswordInput {
    pub token: String,
    pub password: String,
}

#[derive(Clone)]
pub struct ResetPasswordUseCase {
    auth_repository: Arc<dyn AuthRepository>,
    clock: Arc<dyn Clock>,
}

impl ResetPasswordUseCase {
    pub fn new(auth_repository: Arc<dyn AuthRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            auth_repository,
            clock,
        }
    }

    pub async fn execute(&self, input: ResetPasswordInput) -> Result<(), AppError> {
        let token = input.token.trim();
        if token.is_empty() {
            return Err(DomainError::Validation("Reset token is required").into());
        }

        validate_password(&input.password)?;
        let password = input.password;
        let password_hash = tokio::task::spawn_blocking(move || hash_password(&password))
            .await
            .map_err(|_| AppError::Internal)??;

        let reset = self
            .auth_repository
            .reset_password_with_token(&hash_token(token), self.clock.now(), &password_hash)
            .await?;

        if reset {
            Ok(())
        } else {
            Err(DomainError::Validation("Reset link is invalid or expired").into())
        }
    }
}
