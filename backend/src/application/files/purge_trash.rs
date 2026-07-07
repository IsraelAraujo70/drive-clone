use std::sync::Arc;

use chrono::Duration;

use crate::application::AppError;
use crate::application::ports::StorageError;
use crate::application::ports::clock::Clock;
use crate::application::ports::files::FileRepository;
use crate::application::ports::object_storage::ObjectStorage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurgeTrashOutput {
    pub purged_files: usize,
    pub failed_files: usize,
    pub purged_folders: usize,
}

#[derive(Clone)]
pub struct PurgeTrashUseCase {
    file_repository: Arc<dyn FileRepository>,
    storage: Arc<dyn ObjectStorage>,
    clock: Arc<dyn Clock>,
    retention_days: i64,
}

impl PurgeTrashUseCase {
    pub fn new(
        file_repository: Arc<dyn FileRepository>,
        storage: Arc<dyn ObjectStorage>,
        clock: Arc<dyn Clock>,
        retention_days: i64,
    ) -> Self {
        Self {
            file_repository,
            storage,
            clock,
            retention_days,
        }
    }

    pub async fn execute(&self, limit: i64) -> Result<PurgeTrashOutput, AppError> {
        let cutoff = self.clock.now() - Duration::days(self.retention_days.max(0));
        let candidates = self
            .file_repository
            .list_purgeable_files(cutoff, limit.clamp(1, 1000))
            .await?;

        let mut purged_files = 0;
        let mut failed_files = 0;
        for candidate in candidates {
            match self.storage.delete_object(&candidate.object_key).await {
                Ok(()) | Err(StorageError::NotFound) => {
                    if self
                        .file_repository
                        .purge_file(candidate.file_id, cutoff, candidate.purge_claimed_at)
                        .await?
                        .is_some()
                    {
                        purged_files += 1;
                    }
                }
                Err(_) => {
                    self.file_repository
                        .release_purge_claim(candidate.file_id, candidate.purge_claimed_at)
                        .await?;
                    failed_files += 1;
                    tracing::warn!(
                        file_id = %candidate.file_id,
                        object_key = %candidate.object_key,
                        "purge_trash: delete_object failed; keeping row for retry"
                    );
                }
            }
        }

        let purged_folders = self
            .file_repository
            .purge_empty_trashed_folders(cutoff)
            .await?;

        Ok(PurgeTrashOutput {
            purged_files,
            failed_files,
            purged_folders,
        })
    }
}
