use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use uuid::Uuid;

use crate::adapters::http::auth_extractor::AuthenticatedUser;
use crate::adapters::http::dto::{
    CreateUploadRequest, CreateUploadResponse, DownloadResponse, FileResponse, ListFilesResponse,
};
use crate::adapters::http::error::HttpError;
use crate::bootstrap::state::AppState;

pub async fn create_upload(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(request): Json<CreateUploadRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let output = state
        .create_upload
        .execute(&auth.user, request.into())
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(CreateUploadResponse::from(output)),
    ))
}

pub async fn complete_upload(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(file_id): Path<Uuid>,
) -> Result<Json<FileResponse>, HttpError> {
    let file = state.complete_upload.execute(&auth.user, file_id).await?;
    Ok(Json(FileResponse::from(file)))
}

pub async fn list_files(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<ListFilesResponse>, HttpError> {
    let files = state.list_files.execute(&auth.user).await?;
    Ok(Json(ListFilesResponse::from(files)))
}

pub async fn download_file(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(file_id): Path<Uuid>,
) -> Result<Json<DownloadResponse>, HttpError> {
    let output = state.download_file.execute(&auth.user, file_id).await?;
    Ok(Json(DownloadResponse::from(output)))
}
