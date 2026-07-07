use std::sync::Arc;

use crate::application::AppError;
use crate::application::ports::files::{FileRepository, SearchFilesRecord};
use crate::domain::auth::User;
use crate::domain::files::{SearchFileResult, validate_search_query};

const DEFAULT_SEARCH_LIMIT: i64 = 20;
const MAX_SEARCH_LIMIT: i64 = 50;

#[derive(Debug, Clone)]
pub struct SearchFilesInput {
    pub query: String,
    pub include_deleted: bool,
    pub limit: Option<i64>,
}

#[derive(Clone)]
pub struct SearchFilesUseCase {
    file_repository: Arc<dyn FileRepository>,
}

impl SearchFilesUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>) -> Self {
        Self { file_repository }
    }

    pub async fn execute(
        &self,
        user: &User,
        input: SearchFilesInput,
    ) -> Result<Vec<SearchFileResult>, AppError> {
        let query = validate_search_query(&input.query)?;
        let limit = input
            .limit
            .unwrap_or(DEFAULT_SEARCH_LIMIT)
            .clamp(1, MAX_SEARCH_LIMIT);

        self.file_repository
            .search_accessible_files(SearchFilesRecord {
                user_id: user.id,
                query,
                include_deleted: input.include_deleted,
                limit,
            })
            .await
            .map_err(AppError::from)
    }
}
