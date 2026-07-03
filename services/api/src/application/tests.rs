use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use uuid::Uuid;

use crate::adapters::object_storage::fake::FakeObjectStorage;
use crate::application::AppError;
use crate::application::auth::login::{LoginInput, LoginUseCase};
use crate::application::auth::logout::LogoutUseCase;
use crate::application::auth::signup::{SignupInput, SignupUseCase};
use crate::application::files::{
    CompleteUploadUseCase, CreateUploadUseCase, DownloadFileUseCase, ListFilesUseCase,
};
use crate::application::ports::RepositoryError;
use crate::application::ports::auth::{AuthRepository, CreateUserRecord};
use crate::application::ports::clock::{Clock, FixedClock};
use crate::application::ports::files::{CreatePendingFileRecord, FileRepository};
use crate::application::ports::id_generator::SequenceIdGenerator;
use crate::domain::auth::{User, UserWithPassword, hash_password, hash_token};
use crate::domain::error::DomainError;
use crate::domain::files::{DriveFile, FileState, PendingFile, UploadRequest};

fn fixed_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 3, 12, 0, 0).unwrap()
}

fn user(id: Uuid, email: &str, used: i64, quota: i64) -> User {
    User {
        id,
        email: email.to_string(),
        display_name: "Test User".to_string(),
        storage_quota_bytes: quota,
        storage_used_bytes: used,
        created_at: fixed_now(),
    }
}

#[derive(Default)]
struct FakeAuthRepository {
    users: Mutex<HashMap<String, UserWithPassword>>,
    sessions: Mutex<HashMap<String, Uuid>>,
    next_user_id: Mutex<Option<Uuid>>,
    duplicate_on_create: Mutex<bool>,
}

impl FakeAuthRepository {
    fn with_user(email: &str, password: &str) -> Arc<Self> {
        let id = Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap();
        let repo = Arc::new(Self::default());
        repo.users.lock().unwrap().insert(
            email.to_string(),
            UserWithPassword {
                user: user(id, email, 0, 100),
                password_hash: hash_password(password).unwrap(),
            },
        );
        repo
    }
}

#[async_trait]
impl AuthRepository for FakeAuthRepository {
    async fn create_user(&self, input: CreateUserRecord) -> Result<User, RepositoryError> {
        if *self.duplicate_on_create.lock().unwrap()
            || self.users.lock().unwrap().contains_key(&input.email)
        {
            return Err(RepositoryError::DuplicateEmail);
        }
        let id = self
            .next_user_id
            .lock()
            .unwrap()
            .take()
            .unwrap_or_else(Uuid::new_v4);
        let user = user(id, &input.email, 0, 16_106_127_360);
        self.users.lock().unwrap().insert(
            input.email,
            UserWithPassword {
                user: user.clone(),
                password_hash: input.password_hash,
            },
        );
        Ok(user)
    }

    async fn find_user_with_password_by_email(
        &self,
        email: &str,
    ) -> Result<Option<UserWithPassword>, RepositoryError> {
        Ok(self.users.lock().unwrap().get(email).cloned())
    }

    async fn create_session(
        &self,
        user_id: Uuid,
        token_hash: &str,
        _expires_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        self.sessions
            .lock()
            .unwrap()
            .insert(token_hash.to_string(), user_id);
        Ok(())
    }

    async fn find_user_by_session_hash(
        &self,
        token_hash: &str,
        _now: DateTime<Utc>,
    ) -> Result<Option<User>, RepositoryError> {
        let Some(user_id) = self.sessions.lock().unwrap().get(token_hash).copied() else {
            return Ok(None);
        };
        Ok(self
            .users
            .lock()
            .unwrap()
            .values()
            .find(|row| row.user.id == user_id)
            .map(|row| row.user.clone()))
    }

    async fn delete_session(&self, token_hash: &str) -> Result<(), RepositoryError> {
        self.sessions.lock().unwrap().remove(token_hash);
        Ok(())
    }
}

#[derive(Default)]
struct FakeFileRepository {
    pending_bytes: Mutex<i64>,
    pending: Mutex<HashMap<Uuid, PendingFile>>,
    completed: Mutex<HashMap<Uuid, DriveFile>>,
    completed_count: Mutex<usize>,
}

#[async_trait]
impl FileRepository for FakeFileRepository {
    async fn pending_bytes_for_owner(
        &self,
        _owner_id: Uuid,
        cap: i64,
    ) -> Result<i64, RepositoryError> {
        Ok((*self.pending_bytes.lock().unwrap()).min(cap))
    }

    async fn create_pending_file(
        &self,
        input: CreatePendingFileRecord,
    ) -> Result<PendingFile, RepositoryError> {
        let id = Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap();
        let file = PendingFile {
            id,
            size_bytes: input.size_bytes,
            object_key: input.object_key,
            state: FileState::Pending,
        };
        self.pending.lock().unwrap().insert(id, file.clone());
        Ok(file)
    }

    async fn complete_upload_once(
        &self,
        _owner_id: Uuid,
        file_id: Uuid,
        expected_size: i64,
    ) -> Result<DriveFile, RepositoryError> {
        let pending = self
            .pending
            .lock()
            .unwrap()
            .remove(&file_id)
            .ok_or(RepositoryError::NotFound)?;
        if pending.state != FileState::Pending || pending.size_bytes != expected_size {
            return Err(RepositoryError::InvalidState);
        }
        *self.completed_count.lock().unwrap() += 1;
        let file = DriveFile {
            id: file_id,
            filename: "report.txt".to_string(),
            content_type: "text/plain".to_string(),
            size_bytes: pending.size_bytes,
            checksum_sha256: None,
            object_key: pending.object_key,
            state: FileState::Complete,
            created_at: fixed_now(),
            completed_at: Some(fixed_now()),
        };
        self.completed.lock().unwrap().insert(file_id, file.clone());
        Ok(file)
    }

    async fn find_owned_file_for_completion(
        &self,
        _owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<Option<PendingFile>, RepositoryError> {
        Ok(self.pending.lock().unwrap().get(&file_id).cloned())
    }

    async fn list_completed_files(
        &self,
        _owner_id: Uuid,
    ) -> Result<Vec<DriveFile>, RepositoryError> {
        Ok(self.completed.lock().unwrap().values().cloned().collect())
    }

    async fn find_completed_owned_file(
        &self,
        _owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<Option<DriveFile>, RepositoryError> {
        Ok(self.completed.lock().unwrap().get(&file_id).cloned())
    }
}

#[tokio::test]
async fn signup_normalizes_email_and_creates_session() {
    let repo = Arc::new(FakeAuthRepository::default());
    let id = Uuid::parse_str("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa").unwrap();
    *repo.next_user_id.lock().unwrap() = Some(id);
    let clock: Arc<dyn Clock> = Arc::new(FixedClock::new(fixed_now()));
    let use_case = SignupUseCase::new(repo.clone(), clock);

    let response = use_case
        .execute(SignupInput {
            email: "New.User@Example.com".to_string(),
            password: "password123".to_string(),
            display_name: " Test User ".to_string(),
        })
        .await
        .unwrap();

    assert_eq!(response.user.email, "new.user@example.com");
    assert_eq!(response.user.display_name, "Test User");
    assert!(
        repo.sessions
            .lock()
            .unwrap()
            .contains_key(&hash_token(&response.token))
    );
}

#[tokio::test]
async fn signup_maps_duplicate_email() {
    let repo = Arc::new(FakeAuthRepository::default());
    *repo.duplicate_on_create.lock().unwrap() = true;
    let use_case = SignupUseCase::new(repo, Arc::new(FixedClock::new(fixed_now())));

    let error = use_case
        .execute(SignupInput {
            email: "dup@example.com".to_string(),
            password: "password123".to_string(),
            display_name: "Dup".to_string(),
        })
        .await
        .unwrap_err();

    assert_eq!(error, AppError::DuplicateEmail);
}

#[tokio::test]
async fn login_rejects_bad_credentials() {
    let repo = FakeAuthRepository::with_user("user@example.com", "password123");
    let use_case = LoginUseCase::new(repo, Arc::new(FixedClock::new(fixed_now())));

    let error = use_case
        .execute(LoginInput {
            email: "user@example.com".to_string(),
            password: "wrong-password".to_string(),
        })
        .await
        .unwrap_err();

    assert_eq!(error, AppError::InvalidCredentials);
}

#[tokio::test]
async fn logout_revokes_session_hash() {
    let repo = FakeAuthRepository::with_user("user@example.com", "password123");
    let token = "session-token";
    repo.sessions.lock().unwrap().insert(
        hash_token(token),
        Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap(),
    );
    let use_case = LogoutUseCase::new(repo.clone());

    use_case.execute(token).await.unwrap();

    assert!(
        !repo
            .sessions
            .lock()
            .unwrap()
            .contains_key(&hash_token(token))
    );
}

#[tokio::test]
async fn create_upload_enforces_quota_and_max_size() {
    let repo = Arc::new(FakeFileRepository::default());
    *repo.pending_bytes.lock().unwrap() = 6;
    let storage = Arc::new(FakeObjectStorage::default());
    let ids = Arc::new(SequenceIdGenerator::new([Uuid::parse_str(
        "33333333-3333-4333-8333-333333333333",
    )
    .unwrap()]));
    let use_case = CreateUploadUseCase::new(repo, storage, ids, 10, 900);
    let owner = user(Uuid::new_v4(), "quota@example.com", 0, 10);

    let quota_error = use_case
        .execute(
            &owner,
            UploadRequest {
                filename: "report.txt".to_string(),
                content_type: "text/plain".to_string(),
                size_bytes: 5,
                checksum_sha256: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(quota_error, AppError::Domain(DomainError::QuotaExceeded));

    let size_error = use_case
        .execute(
            &owner,
            UploadRequest {
                filename: "report.txt".to_string(),
                content_type: "text/plain".to_string(),
                size_bytes: 11,
                checksum_sha256: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(size_error, AppError::Domain(DomainError::FileTooLarge));
}

#[tokio::test]
async fn complete_upload_checks_storage_length_and_completes_once() {
    let repo = Arc::new(FakeFileRepository::default());
    let file_id = Uuid::parse_str("44444444-4444-4444-8444-444444444444").unwrap();
    repo.pending.lock().unwrap().insert(
        file_id,
        PendingFile {
            id: file_id,
            size_bytes: 12,
            object_key: "owner/object".to_string(),
            state: FileState::Pending,
        },
    );
    let storage = Arc::new(FakeObjectStorage::default());
    storage.put_object("owner/object", 9);
    let use_case = CompleteUploadUseCase::new(repo.clone(), storage.clone());
    let owner = user(Uuid::new_v4(), "owner@example.com", 0, 100);

    assert_eq!(
        use_case.execute(&owner, file_id).await.unwrap_err(),
        AppError::Storage
    );

    storage.put_object("owner/object", 12);
    let completed = use_case.execute(&owner, file_id).await.unwrap();
    assert_eq!(completed.state, FileState::Complete);
    assert_eq!(*repo.completed_count.lock().unwrap(), 1);
}

#[tokio::test]
async fn list_and_download_only_return_completed_files() {
    let repo = Arc::new(FakeFileRepository::default());
    let file_id = Uuid::parse_str("55555555-5555-4555-8555-555555555555").unwrap();
    repo.completed.lock().unwrap().insert(
        file_id,
        DriveFile {
            id: file_id,
            filename: "report.txt".to_string(),
            content_type: "text/plain".to_string(),
            size_bytes: 12,
            checksum_sha256: None,
            object_key: "owner/object".to_string(),
            state: FileState::Complete,
            created_at: fixed_now(),
            completed_at: Some(fixed_now()),
        },
    );
    let storage = Arc::new(FakeObjectStorage::default());
    let owner = user(Uuid::new_v4(), "owner@example.com", 0, 100);

    let files = ListFilesUseCase::new(repo.clone())
        .execute(&owner)
        .await
        .unwrap();
    assert_eq!(files.len(), 1);

    let download = DownloadFileUseCase::new(repo, storage, 900)
        .execute(&owner, file_id)
        .await
        .unwrap();
    assert!(download.download_url.contains("owner/object"));
}
