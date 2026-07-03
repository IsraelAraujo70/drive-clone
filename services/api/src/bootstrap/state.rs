use std::sync::Arc;

use sqlx::PgPool;

use crate::adapters::postgres::{PostgresAuthRepository, PostgresFileRepository};
use crate::application::auth::{GetCurrentUserUseCase, LoginUseCase, LogoutUseCase, SignupUseCase};
use crate::application::files::{
    CompleteUploadUseCase, CreateUploadUseCase, DownloadFileUseCase, ListFilesUseCase,
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
    pub list_files: ListFilesUseCase,
    pub download_file: DownloadFileUseCase,
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
            get_current_user: GetCurrentUserUseCase::new(auth_repository, clock),
            create_upload: CreateUploadUseCase::new(
                file_repository.clone(),
                storage.clone(),
                id_generator,
                max_file_size_bytes,
                presigned_url_ttl_seconds,
            ),
            complete_upload: CompleteUploadUseCase::new(file_repository.clone(), storage.clone()),
            list_files: ListFilesUseCase::new(file_repository.clone()),
            download_file: DownloadFileUseCase::new(
                file_repository,
                storage,
                presigned_url_ttl_seconds,
            ),
        }
    }
}
