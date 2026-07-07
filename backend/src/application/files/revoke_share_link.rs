use std::sync::Arc;

use uuid::Uuid;

use crate::application::AppError;
use crate::application::ports::clock::Clock;
use crate::application::ports::files::FileRepository;
use crate::domain::auth::User;

#[derive(Clone)]
pub struct RevokeShareLinkUseCase {
    file_repository: Arc<dyn FileRepository>,
    clock: Arc<dyn Clock>,
}

impl RevokeShareLinkUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            file_repository,
            clock,
        }
    }

    pub async fn execute(
        &self,
        owner: &User,
        file_id: Uuid,
        link_id: Uuid,
    ) -> Result<(), AppError> {
        self.file_repository
            .revoke_share_link(owner.id, file_id, link_id, self.clock.now())
            .await
            .map_err(AppError::from)
    }
}
