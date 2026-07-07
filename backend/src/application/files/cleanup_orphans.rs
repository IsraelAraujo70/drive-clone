use std::sync::Arc;

use chrono::Duration;

use crate::application::AppError;
use crate::application::ports::StorageError;
use crate::application::ports::clock::Clock;
use crate::application::ports::files::FileRepository;
use crate::application::ports::object_storage::ObjectStorage;

pub const DEFAULT_ORPHAN_MIN_AGE_SECONDS: i64 = 24 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupOrphansOutput {
    pub deleted: usize,
    pub failed: usize,
    pub scanned: usize,
}

#[derive(Clone)]
pub struct CleanupOrphanObjectsUseCase {
    file_repository: Arc<dyn FileRepository>,
    storage: Arc<dyn ObjectStorage>,
    clock: Arc<dyn Clock>,
    min_age_seconds: i64,
}

impl CleanupOrphanObjectsUseCase {
    pub fn new(
        file_repository: Arc<dyn FileRepository>,
        storage: Arc<dyn ObjectStorage>,
        clock: Arc<dyn Clock>,
        min_age_seconds: i64,
    ) -> Self {
        Self {
            file_repository,
            storage,
            clock,
            min_age_seconds,
        }
    }

    pub async fn execute(&self) -> Result<CleanupOrphansOutput, AppError> {
        let known_keys = self.file_repository.all_object_keys().await?;
        let objects = self.storage.list_objects().await?;
        let cutoff = self.clock.now() - Duration::seconds(self.min_age_seconds.max(0));

        let scanned = objects.len();
        let mut deleted = 0;
        let mut failed = 0;

        for object in objects {
            if known_keys.contains(&object.key) || object.last_modified > cutoff {
                continue;
            }

            match self.storage.delete_object(&object.key).await {
                Ok(()) | Err(StorageError::NotFound) => deleted += 1,
                Err(_) => {
                    failed += 1;
                    tracing::warn!(
                        object_key = %object.key,
                        "cleanup_orphan_objects: delete_object failed"
                    );
                }
            }
        }

        Ok(CleanupOrphansOutput {
            deleted,
            failed,
            scanned,
        })
    }
}
