use crate::domain::error::DomainError;

pub fn ensure_quota(
    storage_used_bytes: i64,
    storage_quota_bytes: i64,
    pending_bytes: i64,
    new_file_size: i64,
) -> Result<(), DomainError> {
    if storage_used_bytes
        .checked_add(pending_bytes)
        .and_then(|used| used.checked_add(new_file_size))
        .is_some_and(|projected| projected <= storage_quota_bytes)
    {
        Ok(())
    } else {
        Err(DomainError::QuotaExceeded)
    }
}
