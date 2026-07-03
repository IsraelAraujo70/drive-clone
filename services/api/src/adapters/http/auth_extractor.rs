use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;

use crate::adapters::http::error::HttpError;
use crate::application::AppError;
use crate::bootstrap::state::AppState;
use crate::domain::auth::User;

pub struct AuthenticatedUser {
    pub user: User,
    pub token_hash: String,
}

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = HttpError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized)?;

        let current = state.get_current_user.execute(token).await?;
        Ok(Self {
            user: current.user,
            token_hash: current.token_hash,
        })
    }
}
