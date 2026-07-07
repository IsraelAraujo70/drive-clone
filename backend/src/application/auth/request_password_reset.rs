use std::sync::Arc;

use chrono::Duration;

use crate::application::AppError;
use crate::application::ports::auth::AuthRepository;
use crate::application::ports::clock::Clock;
use crate::application::ports::email::{EmailSender, PasswordResetEmail};
use crate::domain::auth::{generate_token, hash_token, validate_email};

pub const PASSWORD_RESET_TTL_MINUTES: i64 = 60;

#[derive(Debug, Clone)]
pub struct RequestPasswordResetInput {
    pub email: String,
}

#[derive(Clone)]
pub struct RequestPasswordResetUseCase {
    auth_repository: Arc<dyn AuthRepository>,
    email_sender: Arc<dyn EmailSender>,
    clock: Arc<dyn Clock>,
    public_web_url: String,
}

impl RequestPasswordResetUseCase {
    pub fn new(
        auth_repository: Arc<dyn AuthRepository>,
        email_sender: Arc<dyn EmailSender>,
        clock: Arc<dyn Clock>,
        public_web_url: String,
    ) -> Self {
        Self {
            auth_repository,
            email_sender,
            clock,
            public_web_url: public_web_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn execute(&self, input: RequestPasswordResetInput) -> Result<(), AppError> {
        let email = input.email.trim().to_lowercase();
        validate_email(&email)?;

        let Some(user) = self.auth_repository.find_user_by_email(&email).await? else {
            return Ok(());
        };

        let token = generate_token();
        let expires_at = self.clock.now() + Duration::minutes(PASSWORD_RESET_TTL_MINUTES);
        self.auth_repository
            .create_password_reset_token(user.id, &hash_token(&token), expires_at)
            .await?;

        let reset_url = format!("{}/reset-password?token={token}", self.public_web_url);
        self.email_sender
            .send_password_reset(PasswordResetEmail {
                to: user.email,
                display_name: user.display_name,
                reset_url,
                expires_at,
            })
            .await?;

        Ok(())
    }
}
