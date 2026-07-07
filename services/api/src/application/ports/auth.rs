use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::ports::RepositoryError;
use crate::domain::auth::{User, UserWithPassword};

#[derive(Debug, Clone)]
pub struct CreateUserRecord {
    pub email: String,
    pub password_hash: String,
    pub display_name: String,
}

#[async_trait]
pub trait AuthRepository: Send + Sync {
    async fn create_user(&self, input: CreateUserRecord) -> Result<User, RepositoryError>;

    async fn find_user_with_password_by_email(
        &self,
        email: &str,
    ) -> Result<Option<UserWithPassword>, RepositoryError>;

    async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, RepositoryError>;

    async fn create_session(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;

    async fn find_user_by_session_hash(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<User>, RepositoryError>;

    async fn delete_session(&self, token_hash: &str) -> Result<(), RepositoryError>;

    async fn create_password_reset_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;

    async fn reset_password_with_token(
        &self,
        token_hash: &str,
        now: DateTime<Utc>,
        password_hash: &str,
    ) -> Result<bool, RepositoryError>;
}
