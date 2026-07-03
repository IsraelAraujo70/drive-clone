use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::adapters::http::auth_extractor::AuthenticatedUser;
use crate::adapters::http::dto::{AuthResponse, LoginRequest, SignupRequest};
use crate::adapters::http::error::HttpError;
use crate::application::auth::login::LoginInput;
use crate::application::auth::signup::SignupInput;
use crate::bootstrap::state::AppState;
use crate::domain::auth::User;

pub async fn signup(
    State(state): State<AppState>,
    Json(request): Json<SignupRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = state
        .signup
        .execute(SignupInput {
            email: request.email,
            password: request.password,
            display_name: request.display_name,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(AuthResponse::from(response))))
}

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let response = state
        .login
        .execute(LoginInput {
            email: request.email,
            password: request.password,
        })
        .await?;
    Ok((StatusCode::OK, Json(AuthResponse::from(response))))
}

pub async fn logout(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<StatusCode, HttpError> {
    state.logout.execute_hash(&auth.token_hash).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn me(auth: AuthenticatedUser) -> Json<User> {
    Json(auth.user)
}
