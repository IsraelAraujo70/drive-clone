use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use uuid::Uuid;

use crate::adapters::http::auth_extractor::AuthenticatedUser;
use crate::adapters::http::dto::{
    BrowseDriveQuery, CreateFolderRequest, CreateUploadRequest, CreateUploadResponse,
    DownloadResponse, DriveBrowseResponse, FileResponse, FileShareResponse, FolderResponse,
    ListFilesResponse, ListFoldersResponse, ListSharedWithMeResponse, ListSharesResponse,
    SearchFilesQuery, SearchFilesResponse, ShareFileRequest, UpdateFileRequest,
    UpdateFolderRequest,
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

pub async fn create_folder(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(request): Json<CreateFolderRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let folder = state
        .create_folder
        .execute(&auth.user, request.into())
        .await?;
    Ok((StatusCode::CREATED, Json(FolderResponse::from(folder))))
}

pub async fn browse_drive(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<BrowseDriveQuery>,
) -> Result<Json<DriveBrowseResponse>, HttpError> {
    let browse = state
        .browse_folder
        .execute(&auth.user, query.parent_folder_id)
        .await?;
    Ok(Json(DriveBrowseResponse::from(browse)))
}

pub async fn search_files(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<SearchFilesQuery>,
) -> Result<Json<SearchFilesResponse>, HttpError> {
    let normalized_query = query.normalized_query();
    let files = state.search_files.execute(&auth.user, query.into()).await?;
    Ok(Json(SearchFilesResponse::new(normalized_query, files)))
}

pub async fn list_folders(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<ListFoldersResponse>, HttpError> {
    let folders = state.list_folders.execute(&auth.user).await?;
    Ok(Json(ListFoldersResponse::from(folders)))
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

pub async fn update_file(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(file_id): Path<Uuid>,
    Json(request): Json<UpdateFileRequest>,
) -> Result<Json<FileResponse>, HttpError> {
    let file = state
        .update_file
        .execute(&auth.user, request.into_input(file_id))
        .await?;
    Ok(Json(FileResponse::from(file)))
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

pub async fn list_drive_trash(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<DriveBrowseResponse>, HttpError> {
    let browse = state.list_drive_trash.execute(&auth.user).await?;
    Ok(Json(DriveBrowseResponse::from(browse)))
}

pub async fn update_folder(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(folder_id): Path<Uuid>,
    Json(request): Json<UpdateFolderRequest>,
) -> Result<Json<FolderResponse>, HttpError> {
    let folder = state
        .update_folder
        .execute(&auth.user, request.into_input(folder_id))
        .await?;
    Ok(Json(FolderResponse::from(folder)))
}

pub async fn delete_folder(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(folder_id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    state.delete_folder.execute(&auth.user, folder_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn restore_folder(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(folder_id): Path<Uuid>,
) -> Result<Json<FolderResponse>, HttpError> {
    let folder = state.restore_folder.execute(&auth.user, folder_id).await?;
    Ok(Json(FolderResponse::from(folder)))
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
