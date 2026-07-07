use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::application::ports::EmailError;

#[derive(Debug, Clone)]
pub struct PasswordResetEmail {
    pub to: String,
    pub display_name: String,
    pub reset_url: String,
    pub expires_at: DateTime<Utc>,
}

#[async_trait]
pub trait EmailSender: Send + Sync {
    async fn send_password_reset(&self, email: PasswordResetEmail) -> Result<(), EmailError>;
}
