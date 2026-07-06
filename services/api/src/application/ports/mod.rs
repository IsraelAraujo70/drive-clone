pub mod auth;
pub mod clock;
pub mod files;
pub mod id_generator;
pub mod object_storage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryError {
    DuplicateEmail,
    NotFound,
    InvalidState,
    Unexpected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    NotFound,
    Unexpected,
}
