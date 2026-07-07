use std::sync::Arc;

use chrono::Duration;

use crate::application::AppError;
use crate::application::auth::signup::AuthResponse;
use crate::application::ports::auth::AuthRepository;
use crate::application::ports::clock::Clock;
use crate::domain::auth::{
    SESSION_TTL_DAYS, dummy_hash, generate_token, hash_token, verify_password,
};

#[derive(Debug, Clone)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
}

#[derive(Clone)]
pub struct LoginUseCase {
    auth_repository: Arc<dyn AuthRepository>,
    clock: Arc<dyn Clock>,
}

impl LoginUseCase {
    pub fn new(auth_repository: Arc<dyn AuthRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            auth_repository,
            clock,
        }
    }

    pub async fn execute(&self, input: LoginInput) -> Result<AuthResponse, AppError> {
        let email = input.email.trim().to_lowercase();
        let row = self
            .auth_repository
            .find_user_with_password_by_email(&email)
            .await?;

        let password = input.password;
        let (user, verified) = match row {
            Some(row) => {
                let password_hash = row.password_hash;
                let verified =
                    tokio::task::spawn_blocking(move || verify_password(&password, &password_hash))
                        .await
                        .map_err(|_| AppError::Internal)?;
                (Some(row.user), verified)
            }
            None => {
                tokio::task::spawn_blocking(move || verify_password(&password, dummy_hash()))
                    .await
                    .map_err(|_| AppError::Internal)?;
                (None, false)
            }
        };

        let user = user
            .filter(|_| verified)
            .ok_or(AppError::InvalidCredentials)?;
        let token = generate_token();
        let expires_at = self.clock.now() + Duration::days(SESSION_TTL_DAYS);
        self.auth_repository
            .create_session(user.id, &hash_token(&token), expires_at)
            .await?;

        Ok(AuthResponse { user, token })
    }
}
