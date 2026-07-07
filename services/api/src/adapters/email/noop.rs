use async_trait::async_trait;

use crate::application::ports::EmailError;
use crate::application::ports::email::{EmailSender, PasswordResetEmail};

#[derive(Debug, Default)]
pub struct NoopEmailSender;

#[async_trait]
impl EmailSender for NoopEmailSender {
    async fn send_password_reset(&self, email: PasswordResetEmail) -> Result<(), EmailError> {
        tracing::warn!(
            to = %email.to,
            reset_url = %email.reset_url,
            "RESEND_API_KEY is not configured; password reset email was not sent"
        );
        Ok(())
    }
}
