use crate::application::ports::RepositoryError;

pub fn map_sqlx_error(error: sqlx::Error) -> RepositoryError {
    tracing::error!("database error: {error}");
    RepositoryError::Unexpected
}
