pub mod auth;
pub mod files;
pub mod ports;

use crate::application::ports::{RepositoryError, StorageError};
use crate::domain::error::DomainError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    Domain(DomainError),
    DuplicateEmail,
    InvalidCredentials,
    Unauthorized,
    Repository,
    Storage,
    Internal,
}

impl From<DomainError> for AppError {
    fn from(error: DomainError) -> Self {
        match error {
            DomainError::EmailTaken => Self::DuplicateEmail,
            DomainError::InvalidCredentials => Self::InvalidCredentials,
            DomainError::Unauthorized => Self::Unauthorized,
            other => Self::Domain(other),
        }
    }
}

impl From<RepositoryError> for AppError {
    fn from(error: RepositoryError) -> Self {
        match error {
            RepositoryError::DuplicateEmail => Self::DuplicateEmail,
            RepositoryError::NotFound => Self::Domain(DomainError::FileNotFound),
            RepositoryError::InvalidState => Self::Domain(DomainError::InvalidFileState),
            RepositoryError::Unexpected => Self::Repository,
        }
    }
}

impl From<StorageError> for AppError {
    fn from(_: StorageError) -> Self {
        Self::Storage
    }
}

#[cfg(test)]
mod tests;
