use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use uuid::Uuid;

use crate::adapters::http::auth_extractor::AuthenticatedUser;
use crate::adapters::http::dto::{
    CreateUploadRequest, CreateUploadResponse, DownloadResponse, FileResponse, FileShareResponse,
    ListFilesResponse, ListSharedWithMeResponse, ListSharesResponse, ShareFileRequest,
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

pub async fn delete_file(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(file_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    state.delete_file.execute(&auth.user, file_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn restore_file(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(file_id): Path<Uuid>,
) -> Result<Json<FileResponse>, HttpError> {
    let file = state.restore_file.execute(&auth.user, file_id).await?;
    Ok(Json(FileResponse::from(file)))
}

pub async fn list_trash(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<ListFilesResponse>, HttpError> {
    let files = state.list_trash.execute(&auth.user).await?;
    Ok(Json(ListFilesResponse::from(files)))
}

pub async fn share_file(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(file_id): Path<Uuid>,
    Json(request): Json<ShareFileRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let share = state
        .share_file
        .execute(&auth.user, request.into_input(file_id))
        .await?;
    Ok((StatusCode::CREATED, Json(FileShareResponse::from(share))))
}

pub async fn list_shares(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(file_id): Path<Uuid>,
) -> Result<Json<ListSharesResponse>, HttpError> {
    let shares = state.list_shares.execute(&auth.user, file_id).await?;
    Ok(Json(ListSharesResponse::from(shares)))
}

pub async fn revoke_share(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((file_id, grantee_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, HttpError> {
    state
        .revoke_share
        .execute(&auth.user, file_id, grantee_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_shared_with_me(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<ListSharedWithMeResponse>, HttpError> {
    let files = state.list_shared_with_me.execute(&auth.user).await?;
    Ok(Json(ListSharedWithMeResponse::from(files)))
}
