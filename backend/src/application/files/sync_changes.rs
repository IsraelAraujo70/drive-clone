use std::sync::Arc;

use crate::application::AppError;
use crate::application::ports::files::FileRepository;
use crate::domain::auth::User;
use crate::domain::error::DomainError;
use crate::domain::files::ChangeLogEntry;

pub const DEFAULT_SYNC_LIMIT: i64 = 100;
pub const MIN_SYNC_LIMIT: i64 = 1;
pub const MAX_SYNC_LIMIT: i64 = 500;

#[derive(Debug, Clone, Copy)]
pub struct SyncChangesInput {
    pub cursor: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug)]
pub struct SyncChangesOutput {
    pub changes: Vec<ChangeLogEntry>,
    pub next_cursor: i64,
    pub has_more: bool,
}

/// Clamps a requested page size into the accepted `1..=500` window, defaulting
/// to `100` when unset. Non-positive requests clamp up to the minimum.
pub fn clamp_limit(limit: Option<i64>) -> i64 {
    match limit {
        None => DEFAULT_SYNC_LIMIT,
        Some(value) => value.clamp(MIN_SYNC_LIMIT, MAX_SYNC_LIMIT),
    }
}

/// Validates the cursor: absent means "from the beginning" (`0`); a negative
/// cursor is a client error.
pub fn normalize_cursor(cursor: Option<i64>) -> Result<i64, AppError> {
    match cursor {
        None => Ok(0),
        Some(value) if value < 0 => {
            Err(DomainError::Validation("cursor must be greater than or equal to zero").into())
        }
        Some(value) => Ok(value),
    }
}

#[derive(Clone)]
pub struct ListSyncChangesUseCase {
    file_repository: Arc<dyn FileRepository>,
}

impl ListSyncChangesUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>) -> Self {
        Self { file_repository }
    }

    pub async fn execute(
        &self,
        user: &User,
        input: SyncChangesInput,
    ) -> Result<SyncChangesOutput, AppError> {
        let cursor = normalize_cursor(input.cursor)?;
        let limit = clamp_limit(input.limit);

        let changes = self
            .file_repository
            .list_changes(user.id, cursor, limit)
            .await?;

        let has_more = changes.len() as i64 == limit;
        let next_cursor = changes.last().map(|entry| entry.seq).unwrap_or(cursor);

        Ok(SyncChangesOutput {
            changes,
            next_cursor,
            has_more,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_limit_defaults_when_unset() {
        assert_eq!(clamp_limit(None), DEFAULT_SYNC_LIMIT);
    }

    #[test]
    fn clamp_limit_enforces_bounds() {
        assert_eq!(clamp_limit(Some(0)), MIN_SYNC_LIMIT);
        assert_eq!(clamp_limit(Some(-10)), MIN_SYNC_LIMIT);
        assert_eq!(clamp_limit(Some(1)), 1);
        assert_eq!(clamp_limit(Some(250)), 250);
        assert_eq!(clamp_limit(Some(500)), MAX_SYNC_LIMIT);
        assert_eq!(clamp_limit(Some(5000)), MAX_SYNC_LIMIT);
    }

    #[test]
    fn normalize_cursor_defaults_to_zero() {
        assert_eq!(normalize_cursor(None).unwrap(), 0);
        assert_eq!(normalize_cursor(Some(0)).unwrap(), 0);
        assert_eq!(normalize_cursor(Some(42)).unwrap(), 42);
    }

    #[test]
    fn normalize_cursor_rejects_negative() {
        assert!(matches!(
            normalize_cursor(Some(-1)),
            Err(AppError::Domain(DomainError::Validation(_)))
        ));
    }
}
