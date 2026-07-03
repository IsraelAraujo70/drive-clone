#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    Validation(&'static str),
    EmailTaken,
    InvalidCredentials,
    Unauthorized,
    QuotaExceeded,
    FileTooLarge,
    FileNotFound,
    InvalidFileState,
}
