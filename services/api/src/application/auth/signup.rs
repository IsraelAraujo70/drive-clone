use std::sync::Arc;

use chrono::Duration;

use crate::application::AppError;
use crate::application::ports::auth::{AuthRepository, CreateUserRecord};
use crate::application::ports::clock::Clock;
use crate::domain::auth::{
    SESSION_TTL_DAYS, User, generate_token, hash_password, hash_token, validate_display_name,
    validate_email, validate_password,
};

#[derive(Debug, Clone)]
pub struct SignupInput {
    pub email: String,
    pub password: String,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct AuthResponse {
    pub user: User,
    pub token: String,
}

#[derive(Clone)]
pub struct SignupUseCase {
    auth_repository: Arc<dyn AuthRepository>,
    clock: Arc<dyn Clock>,
}

impl SignupUseCase {
    pub fn new(auth_repository: Arc<dyn AuthRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            auth_repository,
            clock,
        }
    }

    pub async fn execute(&self, input: SignupInput) -> Result<AuthResponse, AppError> {
        let email = input.email.trim().to_lowercase();
        let display_name = input.display_name.trim().to_string();
        validate_email(&email)?;
        validate_password(&input.password)?;
        validate_display_name(&display_name)?;

        let password = input.password;
        let password_hash = tokio::task::spawn_blocking(move || hash_password(&password))
            .await
            .map_err(|_| AppError::Internal)??;

        let user = self
            .auth_repository
            .create_user(CreateUserRecord {
                email,
                password_hash,
                display_name,
            })
            .await?;

        let token = generate_token();
        let expires_at = self.clock.now() + Duration::days(SESSION_TTL_DAYS);
        self.auth_repository
            .create_session(user.id, &hash_token(&token), expires_at)
            .await?;

        Ok(AuthResponse { user, token })
    }
}
