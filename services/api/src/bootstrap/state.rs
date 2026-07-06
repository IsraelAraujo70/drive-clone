use std::sync::Arc;

use sqlx::PgPool;

use crate::adapters::postgres::{PostgresAuthRepository, PostgresFileRepository};
use crate::application::auth::{GetCurrentUserUseCase, LoginUseCase, LogoutUseCase, SignupUseCase};
use crate::application::files::{
    BrowseFolderUseCase, CompleteUploadUseCase, CreateFolderUseCase, CreateUploadUseCase,
    DeleteFileUseCase, DeleteFolderUseCase, DownloadFileUseCase, ListDriveTrashUseCase,
    ListFilesUseCase, ListFoldersUseCase, ListSharedWithMeUseCase, ListSharesUseCase,
    ListTrashUseCase, RestoreFileUseCase, RestoreFolderUseCase, RevokeShareUseCase,
    SearchFilesUseCase, ShareFileUseCase, UpdateFileUseCase, UpdateFolderUseCase,
};
use crate::application::ports::auth::AuthRepository;
use crate::application::ports::clock::{Clock, SystemClock};
use crate::application::ports::files::FileRepository;
use crate::application::ports::id_generator::{IdGenerator, UuidGenerator};
use crate::application::ports::object_storage::ObjectStorage;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub signup: SignupUseCase,
    pub login: LoginUseCase,
    pub logout: LogoutUseCase,
    pub get_current_user: GetCurrentUserUseCase,
    pub create_upload: CreateUploadUseCase,
    pub complete_upload: CompleteUploadUseCase,
    pub create_folder: CreateFolderUseCase,
    pub browse_folder: BrowseFolderUseCase,
    pub list_folders: ListFoldersUseCase,
    pub list_files: ListFilesUseCase,
    pub search_files: SearchFilesUseCase,
    pub download_file: DownloadFileUseCase,
    pub update_file: UpdateFileUseCase,
    pub update_folder: UpdateFolderUseCase,
    pub delete_file: DeleteFileUseCase,
    pub delete_folder: DeleteFolderUseCase,
    pub restore_file: RestoreFileUseCase,
    pub restore_folder: RestoreFolderUseCase,
    pub list_trash: ListTrashUseCase,
    pub list_drive_trash: ListDriveTrashUseCase,
    pub share_file: ShareFileUseCase,
    pub list_shares: ListSharesUseCase,
    pub revoke_share: RevokeShareUseCase,
    pub list_shared_with_me: ListSharedWithMeUseCase,
}

impl AppState {
    pub fn from_parts(
        pool: PgPool,
        storage: Arc<dyn ObjectStorage>,
        max_file_size_bytes: i64,
        presigned_url_ttl_seconds: i64,
    ) -> Self {
        let auth_repository: Arc<dyn AuthRepository> =
            Arc::new(PostgresAuthRepository::new(pool.clone()));
        let file_repository: Arc<dyn FileRepository> =
            Arc::new(PostgresFileRepository::new(pool.clone()));
        let clock: Arc<dyn Clock> = Arc::new(SystemClock);
        let id_generator: Arc<dyn IdGenerator> = Arc::new(UuidGenerator);

        Self {
            pool,
            signup: SignupUseCase::new(auth_repository.clone(), clock.clone()),
            login: LoginUseCase::new(auth_repository.clone(), clock.clone()),
            logout: LogoutUseCase::new(auth_repository.clone()),
            get_current_user: GetCurrentUserUseCase::new(auth_repository.clone(), clock),
            create_upload: CreateUploadUseCase::new(
                file_repository.clone(),
                storage.clone(),
                id_generator,
                max_file_size_bytes,
                presigned_url_ttl_seconds,
            ),
            complete_upload: CompleteUploadUseCase::new(file_repository.clone(), storage.clone()),
            create_folder: CreateFolderUseCase::new(file_repository.clone()),
            browse_folder: BrowseFolderUseCase::new(file_repository.clone()),
            list_folders: ListFoldersUseCase::new(file_repository.clone()),
            list_files: ListFilesUseCase::new(file_repository.clone()),
            search_files: SearchFilesUseCase::new(file_repository.clone()),
            delete_file: DeleteFileUseCase::new(file_repository.clone()),
            delete_folder: DeleteFolderUseCase::new(file_repository.clone()),
            restore_file: RestoreFileUseCase::new(file_repository.clone()),
            restore_folder: RestoreFolderUseCase::new(file_repository.clone()),
            list_trash: ListTrashUseCase::new(file_repository.clone()),
            list_drive_trash: ListDriveTrashUseCase::new(file_repository.clone()),
            share_file: ShareFileUseCase::new(file_repository.clone(), auth_repository.clone()),
            list_shares: ListSharesUseCase::new(file_repository.clone()),
            revoke_share: RevokeShareUseCase::new(file_repository.clone()),
            list_shared_with_me: ListSharedWithMeUseCase::new(file_repository.clone()),
            update_file: UpdateFileUseCase::new(file_repository.clone()),
            update_folder: UpdateFolderUseCase::new(file_repository.clone()),
            download_file: DownloadFileUseCase::new(
                file_repository,
                storage,
                presigned_url_ttl_seconds,
            ),
        }
    }
}
