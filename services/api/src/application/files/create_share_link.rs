use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::application::AppError;
use crate::application::ports::clock::Clock;
use crate::application::ports::files::{CreateShareLinkRecord, FileRepository};
use crate::application::ports::id_generator::IdGenerator;
use crate::domain::auth::User;
use crate::domain::error::DomainError;
use crate::domain::files::{generate_share_token, hash_share_token};

#[derive(Debug, Clone)]
pub struct CreateShareLinkInput {
    pub file_id: Uuid,
    pub expires_in_seconds: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct CreateShareLinkOutput {
    pub id: Uuid,
    pub token: String,
    pub url: String,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct CreateShareLinkUseCase {
    file_repository: Arc<dyn FileRepository>,
    id_generator: Arc<dyn IdGenerator>,
    clock: Arc<dyn Clock>,
    public_web_url: String,
}

impl CreateShareLinkUseCase {
    pub fn new(
        file_repository: Arc<dyn FileRepository>,
        id_generator: Arc<dyn IdGenerator>,
        clock: Arc<dyn Clock>,
        public_web_url: String,
    ) -> Self {
        Self {
            file_repository,
            id_generator,
            clock,
            public_web_url,
        }
    }

    pub async fn execute(
        &self,
        owner: &User,
        input: CreateShareLinkInput,
    ) -> Result<CreateShareLinkOutput, AppError> {
        let expires_at = match input.expires_in_seconds {
            Some(seconds) => {
                if seconds <= 0 {
                    return Err(DomainError::Validation(
                        "expires_in_seconds must be greater than zero",
                    )
                    .into());
                }
                Some(self.clock.now() + Duration::seconds(seconds))
            }
            None => None,
        };

        let token = generate_share_token();
        let token_hash = hash_share_token(&token);
        let id = self.id_generator.new_uuid();

        let link = self
            .file_repository
            .create_share_link(CreateShareLinkRecord {
                id,
                owner_id: owner.id,
                file_id: input.file_id,
                token_hash,
                expires_at,
            })
            .await?;

        let url = format!("{}/s/{}", self.public_web_url.trim_end_matches('/'), token);

        Ok(CreateShareLinkOutput {
            id: link.id,
            token,
            url,
            expires_at: link.expires_at,
        })
    }
}
