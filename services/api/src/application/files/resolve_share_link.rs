use std::sync::Arc;

use crate::application::AppError;
use crate::application::ports::clock::Clock;
use crate::application::ports::files::FileRepository;
use crate::application::ports::object_storage::ObjectStorage;
use crate::domain::error::DomainError;
use crate::domain::files::hash_share_token;

#[derive(Debug, Clone)]
pub struct ResolveShareLinkOutput {
    pub filename: String,
    pub size_bytes: i64,
    pub content_type: String,
    pub download_url: String,
}

#[derive(Clone)]
pub struct ResolveShareLinkUseCase {
    file_repository: Arc<dyn FileRepository>,
    storage: Arc<dyn ObjectStorage>,
    clock: Arc<dyn Clock>,
    presigned_url_ttl_seconds: i64,
}

impl ResolveShareLinkUseCase {
    pub fn new(
        file_repository: Arc<dyn FileRepository>,
        storage: Arc<dyn ObjectStorage>,
        clock: Arc<dyn Clock>,
        presigned_url_ttl_seconds: i64,
    ) -> Self {
        Self {
            file_repository,
            storage,
            clock,
            presigned_url_ttl_seconds,
        }
    }

    pub async fn execute(&self, token: &str) -> Result<ResolveShareLinkOutput, AppError> {
        // Uniform 404 for every failure mode (bad token, revoked, expired,
        // trashed file) so the endpoint is not an oracle.
        let token_hash = hash_share_token(token);
        let target = self
            .file_repository
            .resolve_share_link(&token_hash, self.clock.now())
            .await?
            .ok_or(DomainError::FileNotFound)?;

        let presigned = self
            .storage
            .presign_get(&target.object_key, self.presigned_url_ttl_seconds)
            .await?;

        Ok(ResolveShareLinkOutput {
            filename: target.filename,
            size_bytes: target.size_bytes,
            content_type: target.content_type,
            download_url: presigned.url,
        })
    }
}
