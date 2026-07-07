use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::application::AppError;
use crate::domain::error::DomainError;

pub struct HttpError(pub AppError);

impl From<AppError> for HttpError {
    fn from(error: AppError) -> Self {
        Self(error)
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self.0 {
            AppError::Domain(DomainError::Validation(message)) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_error",
                message,
            ),
            AppError::DuplicateEmail | AppError::Domain(DomainError::EmailTaken) => (
                StatusCode::CONFLICT,
                "email_taken",
                "An account with this email already exists",
            ),
            AppError::InvalidCredentials | AppError::Domain(DomainError::InvalidCredentials) => (
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                "Invalid email or password",
            ),
            AppError::Unauthorized | AppError::Domain(DomainError::Unauthorized) => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Missing or invalid session token",
            ),
            AppError::Domain(DomainError::QuotaExceeded) => (
                StatusCode::CONFLICT,
                "quota_exceeded",
                "Not enough storage quota is available",
            ),
            AppError::Domain(DomainError::FileTooLarge) => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "file_too_large",
                "File exceeds the maximum allowed size",
            ),
            AppError::Domain(DomainError::FileNotFound) => (
                StatusCode::NOT_FOUND,
                "file_not_found",
                "File was not found",
            ),
            AppError::Domain(DomainError::UserNotFound) => (
                StatusCode::NOT_FOUND,
                "user_not_found",
                "User was not found",
            ),
            AppError::Domain(DomainError::InvalidFileState) => (
                StatusCode::CONFLICT,
                "invalid_file_state",
                "File is not in the expected state",
            ),
            AppError::Storage => (
                StatusCode::BAD_GATEWAY,
                "storage_error",
                "Object storage could not satisfy the request",
            ),
            AppError::Email => (
                StatusCode::BAD_GATEWAY,
                "email_error",
                "Email delivery could not satisfy the request",
            ),
            AppError::Repository | AppError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "Something went wrong",
            ),
        };

        (status, Json(json!({"error": code, "message": message}))).into_response()
    }
}
