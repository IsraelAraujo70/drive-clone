use std::sync::Arc;

use crate::application::AppError;
use crate::application::ports::files::{FileRepository, QuotaDivergence};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileQuotaOutput {
    pub divergences: Vec<QuotaDivergence>,
}

impl ReconcileQuotaOutput {
    pub fn corrected(&self) -> usize {
        self.divergences.len()
    }
}

#[derive(Clone)]
pub struct ReconcileQuotaUseCase {
    file_repository: Arc<dyn FileRepository>,
    batch_size: i64,
}

impl ReconcileQuotaUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>, batch_size: i64) -> Self {
        Self {
            file_repository,
            batch_size: batch_size.clamp(1, 10_000),
        }
    }

    pub async fn execute(&self) -> Result<ReconcileQuotaOutput, AppError> {
        let mut after_id = None;
        let mut divergences = Vec::new();

        loop {
            let batch = self
                .file_repository
                .reconcile_quota(after_id, self.batch_size)
                .await?;
            for divergence in &batch.divergences {
                tracing::warn!(
                    owner_id = %divergence.owner_id,
                    previous = divergence.previous,
                    corrected = divergence.corrected,
                    "reconcile_quota: corrected storage_used_bytes divergence"
                );
            }
            divergences.extend(batch.divergences);
            match batch.last_user_id {
                Some(id) => after_id = Some(id),
                None => break,
            }
        }

        Ok(ReconcileQuotaOutput { divergences })
    }
}
