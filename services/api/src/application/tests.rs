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
    BrowseFolderUseCase, CompleteUploadUseCase, CreateFolderInput, CreateFolderUseCase,
    CreateResumableUploadUseCase, CreateUploadUseCase, DeleteFileUseCase, DeleteFolderUseCase,
    DownloadFileUseCase, FinalizeResumableUploadUseCase, GetUploadStatusUseCase,
    ListDriveTrashUseCase, ListFilesUseCase, ListFoldersUseCase, ListPendingUploadsUseCase,
    ListSharedWithMeUseCase, ListSharesUseCase, ListTrashUseCase, MIN_RESUMABLE_PART_SIZE_BYTES,
    PresignUploadPartInput, PresignUploadPartUseCase, RecordUploadPartInput,
    RecordUploadPartUseCase, RestoreFileUseCase, RestoreFolderUseCase, RevokeShareUseCase,
    SearchFilesInput, SearchFilesUseCase, ShareFileInput, ShareFileUseCase, UpdateFileInput,
    UpdateFileUseCase, UpdateFolderInput, UpdateFolderUseCase,
};
use crate::application::ports::RepositoryError;
use crate::application::ports::auth::{AuthRepository, CreateUserRecord};
use crate::application::ports::clock::{Clock, FixedClock};
use crate::application::ports::files::{
    CreateFolderRecord, CreatePendingFileRecord, CreateResumableUploadRecord, ExpiredUploadRecord,
    FileRepository, RecordUploadPartRecord, SearchFilesRecord, UpdateFileRecord,
    UpdateFolderRecord,
};
use crate::application::ports::id_generator::SequenceIdGenerator;
use crate::domain::auth::{User, UserWithPassword, hash_password, hash_token};
use crate::domain::error::DomainError;
use crate::domain::files::{
    ChangeLogEntry, DriveBrowse, DriveFile, FileShare, FileState, FileUser, Folder, PendingFile,
    PendingUpload, ResumableUploadRequest, ResumableUploadSession, SearchAccess, SearchFileResult,
    SharedFile, UploadPart, UploadRequest,
};

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

fn completed_file(id: Uuid, object_key: &str) -> DriveFile {
    DriveFile {
        id,
        filename: "report.txt".to_string(),
        parent_folder_id: None,
        content_type: "text/plain".to_string(),
        size_bytes: 12,
        checksum_sha256: None,
        object_key: object_key.to_string(),
        state: FileState::Complete,
        created_at: fixed_now(),
        updated_at: fixed_now(),
        completed_at: Some(fixed_now()),
        deleted_at: None,
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
        repo.add_user(user(id, email, 0, 100), password);
        repo
    }

    fn add_user(&self, user: User, password: &str) {
        self.users.lock().unwrap().insert(
            user.email.clone(),
            UserWithPassword {
                user,
                password_hash: hash_password(password).unwrap(),
            },
        );
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

    async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, RepositoryError> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .get(email)
            .map(|row| row.user.clone()))
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
    pending_owners: Mutex<HashMap<Uuid, Uuid>>,
    pending_parents: Mutex<HashMap<Uuid, Option<Uuid>>>,
    resumable: Mutex<HashMap<Uuid, ResumableUploadSession>>,
    upload_parts: Mutex<HashMap<(Uuid, i32), UploadPart>>,
    completed: Mutex<HashMap<Uuid, DriveFile>>,
    owners: Mutex<HashMap<Uuid, Uuid>>,
    folders: Mutex<HashMap<Uuid, Folder>>,
    folder_owners: Mutex<HashMap<Uuid, Uuid>>,
    users: Mutex<HashMap<Uuid, FileUser>>,
    shares: Mutex<HashMap<(Uuid, Uuid), DateTime<Utc>>>,
    completed_count: Mutex<usize>,
    next_folder_id: Mutex<Option<Uuid>>,
}

impl FakeFileRepository {
    fn insert_user(&self, user: &User) {
        self.users.lock().unwrap().insert(
            user.id,
            FileUser {
                id: user.id,
                email: user.email.clone(),
                display_name: user.display_name.clone(),
            },
        );
    }

    fn insert_completed(&self, owner: &User, file: DriveFile) {
        self.insert_user(owner);
        self.owners.lock().unwrap().insert(file.id, owner.id);
        self.completed.lock().unwrap().insert(file.id, file);
    }

    fn insert_folder(&self, owner: &User, folder: Folder) {
        self.insert_user(owner);
        self.folder_owners
            .lock()
            .unwrap()
            .insert(folder.id, owner.id);
        self.folders.lock().unwrap().insert(folder.id, folder);
    }

    fn active_parent_belongs_to(&self, owner_id: Uuid, parent_id: Option<Uuid>) -> bool {
        let Some(parent_id) = parent_id else {
            return true;
        };
        self.folder_owners.lock().unwrap().get(&parent_id).copied() == Some(owner_id)
            && self
                .folders
                .lock()
                .unwrap()
                .get(&parent_id)
                .is_some_and(|folder| folder.deleted_at.is_none())
    }

    fn folder_descendants(&self, folder_id: Uuid) -> Vec<Uuid> {
        let folders = self.folders.lock().unwrap();
        let mut descendants = Vec::new();
        let mut stack = vec![folder_id];
        while let Some(current) = stack.pop() {
            if current != folder_id {
                descendants.push(current);
            }
            for folder in folders.values() {
                if folder.parent_folder_id == Some(current) {
                    stack.push(folder.id);
                }
            }
        }
        descendants
    }
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
        if !self.active_parent_belongs_to(input.owner_id, input.parent_folder_id) {
            return Err(RepositoryError::NotFound);
        }
        let id = Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap();
        let file = PendingFile {
            id,
            size_bytes: input.size_bytes,
            object_key: input.object_key,
            state: FileState::Pending,
        };
        self.pending.lock().unwrap().insert(id, file.clone());
        self.pending_owners
            .lock()
            .unwrap()
            .insert(id, input.owner_id);
        self.pending_parents
            .lock()
            .unwrap()
            .insert(id, input.parent_folder_id);
        Ok(file)
    }

    async fn create_resumable_upload(
        &self,
        input: CreateResumableUploadRecord,
    ) -> Result<ResumableUploadSession, RepositoryError> {
        if !self.active_parent_belongs_to(input.owner_id, input.parent_folder_id) {
            return Err(RepositoryError::NotFound);
        }
        let id = Uuid::parse_str("33333333-3333-4333-8333-333333333333").unwrap();
        let session = ResumableUploadSession {
            file_id: id,
            owner_id: input.owner_id,
            filename: input.filename,
            parent_folder_id: input.parent_folder_id,
            content_type: input.content_type,
            size_bytes: input.size_bytes,
            checksum_sha256: input.checksum_sha256,
            object_key: input.object_key,
            multipart_upload_id: input.multipart_upload_id,
            part_size_bytes: input.part_size_bytes,
            state: FileState::Pending,
            upload_expires_at: input.upload_expires_at,
            created_at: fixed_now(),
            updated_at: fixed_now(),
            completed_at: None,
            parts: Vec::new(),
        };
        self.resumable.lock().unwrap().insert(id, session.clone());
        Ok(session)
    }

    async fn find_resumable_upload(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<Option<ResumableUploadSession>, RepositoryError> {
        let Some(mut session) = self.resumable.lock().unwrap().get(&file_id).cloned() else {
            return Ok(None);
        };
        if session.owner_id != owner_id {
            return Ok(None);
        }
        let mut parts = self
            .upload_parts
            .lock()
            .unwrap()
            .iter()
            .filter_map(|((part_file_id, _), part)| {
                (*part_file_id == file_id).then_some(part.clone())
            })
            .collect::<Vec<_>>();
        parts.sort_by_key(|part| part.part_number);
        session.parts = parts;
        Ok(Some(session))
    }

    async fn list_pending_resumable_uploads(
        &self,
        owner_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<Vec<PendingUpload>, RepositoryError> {
        let upload_parts = self.upload_parts.lock().unwrap();
        let mut uploads = self
            .resumable
            .lock()
            .unwrap()
            .values()
            .filter(|session| {
                session.owner_id == owner_id
                    && session.state == FileState::Pending
                    && session.upload_expires_at > now
            })
            .map(|session| {
                let parts_received = upload_parts
                    .keys()
                    .filter(|(file_id, _)| *file_id == session.file_id)
                    .count() as i64;
                PendingUpload {
                    file_id: session.file_id,
                    filename: session.filename.clone(),
                    parent_folder_id: session.parent_folder_id,
                    size_bytes: session.size_bytes,
                    part_size_bytes: session.part_size_bytes,
                    checksum_sha256: session.checksum_sha256.clone(),
                    parts_received,
                    expires_at: session.upload_expires_at,
                }
            })
            .collect::<Vec<_>>();
        uploads.sort_by_key(|upload| std::cmp::Reverse(upload.expires_at));
        Ok(uploads)
    }

    async fn record_upload_part(
        &self,
        input: RecordUploadPartRecord,
    ) -> Result<UploadPart, RepositoryError> {
        let session = self
            .resumable
            .lock()
            .unwrap()
            .get(&input.file_id)
            .cloned()
            .ok_or(RepositoryError::NotFound)?;
        if session.owner_id != input.owner_id || session.state != FileState::Pending {
            return Err(RepositoryError::InvalidState);
        }
        let part = UploadPart {
            part_number: input.part_number,
            size_bytes: input.size_bytes,
            etag: input.etag,
        };
        self.upload_parts
            .lock()
            .unwrap()
            .insert((input.file_id, input.part_number), part.clone());
        Ok(part)
    }

    async fn complete_resumable_upload_once(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
        expected_size: i64,
    ) -> Result<DriveFile, RepositoryError> {
        let mut resumable = self.resumable.lock().unwrap();
        let session = resumable
            .get_mut(&file_id)
            .filter(|session| session.owner_id == owner_id)
            .ok_or(RepositoryError::NotFound)?;
        if session.state != FileState::Pending || session.size_bytes != expected_size {
            return Err(RepositoryError::InvalidState);
        }
        session.state = FileState::Complete;
        session.completed_at = Some(fixed_now());
        let file = DriveFile {
            id: file_id,
            filename: session.filename.clone(),
            parent_folder_id: session.parent_folder_id,
            content_type: session.content_type.clone(),
            size_bytes: session.size_bytes,
            checksum_sha256: session.checksum_sha256.clone(),
            object_key: session.object_key.clone(),
            state: FileState::Complete,
            created_at: session.created_at,
            updated_at: fixed_now(),
            completed_at: Some(fixed_now()),
            deleted_at: None,
        };
        self.owners.lock().unwrap().insert(file_id, owner_id);
        self.completed.lock().unwrap().insert(file_id, file.clone());
        Ok(file)
    }

    async fn expire_resumable_uploads(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<ExpiredUploadRecord>, RepositoryError> {
        let mut expired = Vec::new();
        for session in self.resumable.lock().unwrap().values_mut() {
            if expired.len() >= limit as usize {
                break;
            }
            if session.state == FileState::Pending && session.upload_expires_at < now {
                session.state = FileState::Expired;
                expired.push(ExpiredUploadRecord {
                    file_id: session.file_id,
                    object_key: session.object_key.clone(),
                    multipart_upload_id: session.multipart_upload_id.clone(),
                });
            }
        }
        Ok(expired)
    }

    async fn create_folder(&self, input: CreateFolderRecord) -> Result<Folder, RepositoryError> {
        if !self.active_parent_belongs_to(input.owner_id, input.parent_folder_id) {
            return Err(RepositoryError::NotFound);
        }
        let id = self
            .next_folder_id
            .lock()
            .unwrap()
            .take()
            .unwrap_or_else(Uuid::new_v4);
        let folder = Folder {
            id,
            name: input.name,
            parent_folder_id: input.parent_folder_id,
            created_at: fixed_now(),
            updated_at: fixed_now(),
            deleted_at: None,
        };
        self.folder_owners
            .lock()
            .unwrap()
            .insert(id, input.owner_id);
        self.folders.lock().unwrap().insert(id, folder.clone());
        Ok(folder)
    }

    async fn browse_folder(
        &self,
        owner_id: Uuid,
        parent_folder_id: Option<Uuid>,
    ) -> Result<DriveBrowse, RepositoryError> {
        if !self.active_parent_belongs_to(owner_id, parent_folder_id) {
            return Err(RepositoryError::NotFound);
        }
        let folder_owners = self.folder_owners.lock().unwrap();
        let folders = self.folders.lock().unwrap();
        let owners = self.owners.lock().unwrap();
        let completed = self.completed.lock().unwrap();
        Ok(DriveBrowse {
            parent_folder_id,
            breadcrumbs: Vec::new(),
            folders: folders
                .values()
                .filter(|folder| {
                    folder_owners.get(&folder.id).copied() == Some(owner_id)
                        && folder.parent_folder_id == parent_folder_id
                        && folder.deleted_at.is_none()
                })
                .cloned()
                .collect(),
            files: completed
                .values()
                .filter(|file| {
                    owners.get(&file.id).copied() == Some(owner_id)
                        && file.parent_folder_id == parent_folder_id
                        && file.deleted_at.is_none()
                })
                .cloned()
                .collect(),
        })
    }

    async fn list_active_folders(&self, owner_id: Uuid) -> Result<Vec<Folder>, RepositoryError> {
        let folder_owners = self.folder_owners.lock().unwrap();
        Ok(self
            .folders
            .lock()
            .unwrap()
            .values()
            .filter(|folder| {
                folder_owners.get(&folder.id).copied() == Some(owner_id)
                    && folder.deleted_at.is_none()
            })
            .cloned()
            .collect())
    }

    async fn complete_upload_once(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
        expected_size: i64,
    ) -> Result<DriveFile, RepositoryError> {
        let pending = self
            .pending
            .lock()
            .unwrap()
            .remove(&file_id)
            .ok_or(RepositoryError::NotFound)?;
        let pending_owner = self
            .pending_owners
            .lock()
            .unwrap()
            .remove(&file_id)
            .ok_or(RepositoryError::NotFound)?;
        let parent_folder_id = self
            .pending_parents
            .lock()
            .unwrap()
            .remove(&file_id)
            .unwrap_or(None);
        if pending_owner != owner_id {
            return Err(RepositoryError::NotFound);
        }
        if pending.state != FileState::Pending || pending.size_bytes != expected_size {
            return Err(RepositoryError::InvalidState);
        }
        *self.completed_count.lock().unwrap() += 1;
        let file = DriveFile {
            id: file_id,
            filename: "report.txt".to_string(),
            parent_folder_id,
            content_type: "text/plain".to_string(),
            size_bytes: pending.size_bytes,
            checksum_sha256: None,
            object_key: pending.object_key,
            state: FileState::Complete,
            created_at: fixed_now(),
            updated_at: fixed_now(),
            completed_at: Some(fixed_now()),
            deleted_at: None,
        };
        self.owners.lock().unwrap().insert(file_id, owner_id);
        self.completed.lock().unwrap().insert(file_id, file.clone());
        Ok(file)
    }

    async fn find_owned_file_for_completion(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<Option<PendingFile>, RepositoryError> {
        if self.pending_owners.lock().unwrap().get(&file_id).copied() != Some(owner_id) {
            return Ok(None);
        }
        Ok(self.pending.lock().unwrap().get(&file_id).cloned())
    }

    async fn list_completed_files(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<DriveFile>, RepositoryError> {
        let owners = self.owners.lock().unwrap();
        Ok(self
            .completed
            .lock()
            .unwrap()
            .values()
            .filter(|file| {
                owners.get(&file.id).copied() == Some(owner_id) && file.deleted_at.is_none()
            })
            .cloned()
            .collect())
    }

    async fn find_completed_owned_file(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<Option<DriveFile>, RepositoryError> {
        if self.owners.lock().unwrap().get(&file_id).copied() != Some(owner_id) {
            return Ok(None);
        }
        Ok(self
            .completed
            .lock()
            .unwrap()
            .get(&file_id)
            .filter(|file| file.deleted_at.is_none())
            .cloned())
    }

    async fn update_owned_file(
        &self,
        input: UpdateFileRecord,
    ) -> Result<DriveFile, RepositoryError> {
        if self.owners.lock().unwrap().get(&input.file_id).copied() != Some(input.owner_id) {
            return Err(RepositoryError::NotFound);
        }
        if !self.active_parent_belongs_to(input.owner_id, input.parent_folder_id.flatten()) {
            return Err(RepositoryError::NotFound);
        }
        let mut completed = self.completed.lock().unwrap();
        let file = completed
            .get_mut(&input.file_id)
            .filter(|file| file.state == FileState::Complete && file.deleted_at.is_none())
            .ok_or(RepositoryError::NotFound)?;
        if let Some(filename) = input.filename {
            file.filename = filename;
        }
        if let Some(parent_folder_id) = input.parent_folder_id {
            file.parent_folder_id = parent_folder_id;
        }
        file.updated_at = fixed_now();
        Ok(file.clone())
    }

    async fn update_owned_folder(
        &self,
        input: UpdateFolderRecord,
    ) -> Result<Folder, RepositoryError> {
        if self
            .folder_owners
            .lock()
            .unwrap()
            .get(&input.folder_id)
            .copied()
            != Some(input.owner_id)
        {
            return Err(RepositoryError::NotFound);
        }
        if !self.active_parent_belongs_to(input.owner_id, input.parent_folder_id.flatten()) {
            return Err(RepositoryError::NotFound);
        }
        if let Some(parent_folder_id) = input.parent_folder_id.flatten() {
            if parent_folder_id == input.folder_id
                || self
                    .folder_descendants(input.folder_id)
                    .contains(&parent_folder_id)
            {
                return Err(RepositoryError::InvalidState);
            }
        }
        let mut folders = self.folders.lock().unwrap();
        let folder = folders
            .get_mut(&input.folder_id)
            .filter(|folder| folder.deleted_at.is_none())
            .ok_or(RepositoryError::NotFound)?;
        if let Some(name) = input.name {
            folder.name = name;
        }
        if let Some(parent_folder_id) = input.parent_folder_id {
            folder.parent_folder_id = parent_folder_id;
        }
        folder.updated_at = fixed_now();
        Ok(folder.clone())
    }

    async fn find_downloadable_file(
        &self,
        user_id: Uuid,
        file_id: Uuid,
    ) -> Result<Option<DriveFile>, RepositoryError> {
        let owner_allowed = self.owners.lock().unwrap().get(&file_id).copied() == Some(user_id);
        let share_allowed = self
            .shares
            .lock()
            .unwrap()
            .contains_key(&(file_id, user_id));
        if !owner_allowed && !share_allowed {
            return Ok(None);
        }
        Ok(self
            .completed
            .lock()
            .unwrap()
            .get(&file_id)
            .filter(|file| file.deleted_at.is_none())
            .cloned())
    }

    async fn soft_delete_owned_file(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<(), RepositoryError> {
        if self.owners.lock().unwrap().get(&file_id).copied() != Some(owner_id) {
            return Err(RepositoryError::NotFound);
        }
        let mut completed = self.completed.lock().unwrap();
        let file = completed
            .get_mut(&file_id)
            .filter(|file| file.state == FileState::Complete && file.deleted_at.is_none())
            .ok_or(RepositoryError::NotFound)?;
        file.deleted_at = Some(fixed_now());
        Ok(())
    }

    async fn restore_owned_file(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<DriveFile, RepositoryError> {
        if self.owners.lock().unwrap().get(&file_id).copied() != Some(owner_id) {
            return Err(RepositoryError::NotFound);
        }
        let mut completed = self.completed.lock().unwrap();
        let file = completed
            .get_mut(&file_id)
            .filter(|file| file.deleted_at.is_some())
            .ok_or(RepositoryError::NotFound)?;
        file.deleted_at = None;
        Ok(file.clone())
    }

    async fn list_trash(&self, owner_id: Uuid) -> Result<Vec<DriveFile>, RepositoryError> {
        let owners = self.owners.lock().unwrap();
        Ok(self
            .completed
            .lock()
            .unwrap()
            .values()
            .filter(|file| {
                owners.get(&file.id).copied() == Some(owner_id) && file.deleted_at.is_some()
            })
            .cloned()
            .collect())
    }

    async fn soft_delete_owned_folder_tree(
        &self,
        owner_id: Uuid,
        folder_id: Uuid,
    ) -> Result<(), RepositoryError> {
        if self.folder_owners.lock().unwrap().get(&folder_id).copied() != Some(owner_id) {
            return Err(RepositoryError::NotFound);
        }
        {
            let folders = self.folders.lock().unwrap();
            if !folders
                .get(&folder_id)
                .is_some_and(|folder| folder.deleted_at.is_none())
            {
                return Err(RepositoryError::NotFound);
            }
        }

        let mut tree = self.folder_descendants(folder_id);
        tree.push(folder_id);

        {
            let mut folders = self.folders.lock().unwrap();
            for id in &tree {
                if let Some(folder) = folders
                    .get_mut(id)
                    .filter(|folder| folder.deleted_at.is_none())
                {
                    folder.deleted_at = Some(fixed_now());
                    folder.updated_at = fixed_now();
                }
            }
        }

        let mut completed = self.completed.lock().unwrap();
        for file in completed.values_mut() {
            if file.deleted_at.is_none()
                && file.parent_folder_id.is_some_and(|id| tree.contains(&id))
            {
                file.deleted_at = Some(fixed_now());
                file.updated_at = fixed_now();
            }
        }
        Ok(())
    }

    async fn restore_owned_folder_tree(
        &self,
        owner_id: Uuid,
        folder_id: Uuid,
    ) -> Result<Folder, RepositoryError> {
        if self.folder_owners.lock().unwrap().get(&folder_id).copied() != Some(owner_id) {
            return Err(RepositoryError::NotFound);
        }
        {
            let folders = self.folders.lock().unwrap();
            let folder = folders.get(&folder_id).ok_or(RepositoryError::NotFound)?;
            if folder.deleted_at.is_none() {
                return Err(RepositoryError::NotFound);
            }
            if folder.parent_folder_id.is_some_and(|parent_id| {
                folders
                    .get(&parent_id)
                    .is_some_and(|parent| parent.deleted_at.is_some())
            }) {
                return Err(RepositoryError::InvalidState);
            }
        }

        let mut tree = self.folder_descendants(folder_id);
        tree.push(folder_id);
        {
            let mut folders = self.folders.lock().unwrap();
            for id in &tree {
                if let Some(folder) = folders.get_mut(id) {
                    folder.deleted_at = None;
                    folder.updated_at = fixed_now();
                }
            }
        }

        let mut completed = self.completed.lock().unwrap();
        for file in completed.values_mut() {
            if file.parent_folder_id.is_some_and(|id| tree.contains(&id)) {
                file.deleted_at = None;
                file.updated_at = fixed_now();
            }
        }

        self.folders
            .lock()
            .unwrap()
            .get(&folder_id)
            .cloned()
            .ok_or(RepositoryError::NotFound)
    }

    async fn list_drive_trash(&self, owner_id: Uuid) -> Result<DriveBrowse, RepositoryError> {
        let folder_owners = self.folder_owners.lock().unwrap();
        let owners = self.owners.lock().unwrap();
        Ok(DriveBrowse {
            parent_folder_id: None,
            breadcrumbs: Vec::new(),
            folders: self
                .folders
                .lock()
                .unwrap()
                .values()
                .filter(|folder| {
                    folder_owners.get(&folder.id).copied() == Some(owner_id)
                        && folder.deleted_at.is_some()
                })
                .cloned()
                .collect(),
            files: self
                .completed
                .lock()
                .unwrap()
                .values()
                .filter(|file| {
                    owners.get(&file.id).copied() == Some(owner_id) && file.deleted_at.is_some()
                })
                .cloned()
                .collect(),
        })
    }

    async fn create_share(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
        grantee_id: Uuid,
    ) -> Result<FileShare, RepositoryError> {
        if self.owners.lock().unwrap().get(&file_id).copied() != Some(owner_id) {
            return Err(RepositoryError::NotFound);
        }
        if !self
            .completed
            .lock()
            .unwrap()
            .get(&file_id)
            .is_some_and(|file| file.state == FileState::Complete && file.deleted_at.is_none())
        {
            return Err(RepositoryError::NotFound);
        }
        self.shares
            .lock()
            .unwrap()
            .entry((file_id, grantee_id))
            .or_insert_with(fixed_now);
        let grantee = self
            .users
            .lock()
            .unwrap()
            .get(&grantee_id)
            .cloned()
            .ok_or(RepositoryError::NotFound)?;
        Ok(FileShare {
            file_id,
            grantee,
            created_at: fixed_now(),
        })
    }

    async fn list_shares(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<Vec<FileShare>, RepositoryError> {
        if self.owners.lock().unwrap().get(&file_id).copied() != Some(owner_id) {
            return Err(RepositoryError::NotFound);
        }
        let users = self.users.lock().unwrap();
        Ok(self
            .shares
            .lock()
            .unwrap()
            .iter()
            .filter(|((shared_file_id, _), _)| *shared_file_id == file_id)
            .map(|((_, grantee_id), created_at)| FileShare {
                file_id,
                grantee: users.get(grantee_id).cloned().unwrap(),
                created_at: *created_at,
            })
            .collect())
    }

    async fn revoke_share(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
        grantee_id: Uuid,
    ) -> Result<(), RepositoryError> {
        if self.owners.lock().unwrap().get(&file_id).copied() != Some(owner_id) {
            return Err(RepositoryError::NotFound);
        }
        if self
            .shares
            .lock()
            .unwrap()
            .remove(&(file_id, grantee_id))
            .is_some()
        {
            Ok(())
        } else {
            Err(RepositoryError::NotFound)
        }
    }

    async fn list_shared_with_me(
        &self,
        grantee_id: Uuid,
    ) -> Result<Vec<SharedFile>, RepositoryError> {
        let completed = self.completed.lock().unwrap();
        let owners = self.owners.lock().unwrap();
        let users = self.users.lock().unwrap();
        Ok(self
            .shares
            .lock()
            .unwrap()
            .keys()
            .filter(|(_, share_grantee_id)| *share_grantee_id == grantee_id)
            .filter_map(|(file_id, _)| {
                let file = completed
                    .get(file_id)
                    .filter(|file| file.deleted_at.is_none())?;
                let owner_id = owners.get(file_id)?;
                Some(SharedFile {
                    file: file.clone(),
                    owner: users.get(owner_id).cloned().unwrap(),
                })
            })
            .collect())
    }

    async fn search_accessible_files(
        &self,
        input: SearchFilesRecord,
    ) -> Result<Vec<SearchFileResult>, RepositoryError> {
        let query = input.query.to_lowercase();
        let completed = self.completed.lock().unwrap();
        let owners = self.owners.lock().unwrap();
        let users = self.users.lock().unwrap();
        let shares = self.shares.lock().unwrap();
        let mut results: Vec<SearchFileResult> = completed
            .iter()
            .filter_map(|(file_id, file)| {
                if file.state != FileState::Complete
                    || !file.filename.to_lowercase().contains(&query)
                {
                    return None;
                }

                if owners.get(file_id).copied() == Some(input.user_id)
                    && (input.include_deleted || file.deleted_at.is_none())
                {
                    return Some(SearchFileResult {
                        file: file.clone(),
                        access: SearchAccess::Owned,
                        owner: None,
                    });
                }

                if file.deleted_at.is_none() && shares.contains_key(&(*file_id, input.user_id)) {
                    let owner_id = owners.get(file_id)?;
                    return Some(SearchFileResult {
                        file: file.clone(),
                        access: SearchAccess::Shared,
                        owner: users.get(owner_id).cloned(),
                    });
                }

                None
            })
            .collect();

        results.sort_by(|a, b| {
            let a_name = a.file.filename.to_lowercase();
            let b_name = b.file.filename.to_lowercase();
            let a_rank = if a_name == query {
                0
            } else if a_name.starts_with(&query) {
                1
            } else {
                2
            };
            let b_rank = if b_name == query {
                0
            } else if b_name.starts_with(&query) {
                1
            } else {
                2
            };
            a_rank
                .cmp(&b_rank)
                .then_with(|| b.file.completed_at.cmp(&a.file.completed_at))
                .then_with(|| b.file.id.cmp(&a.file.id))
        });
        results.truncate(input.limit as usize);
        Ok(results)
    }

    async fn list_changes(
        &self,
        _owner_id: Uuid,
        _after_seq: i64,
        _limit: i64,
    ) -> Result<Vec<ChangeLogEntry>, RepositoryError> {
        Ok(Vec::new())
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
                parent_folder_id: None,
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
                parent_folder_id: None,
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
    repo.pending_owners
        .lock()
        .unwrap()
        .insert(file_id, owner.id);

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
async fn resumable_upload_records_parts_and_finalizes_complete_object() {
    let repo = Arc::new(FakeFileRepository::default());
    let storage = Arc::new(FakeObjectStorage::default());
    let ids = Arc::new(SequenceIdGenerator::new([Uuid::parse_str(
        "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
    )
    .unwrap()]));
    let clock = Arc::new(FixedClock::new(fixed_now()));
    let part_size = MIN_RESUMABLE_PART_SIZE_BYTES;
    let total_size = part_size * 2 + 2;
    let owner = user(Uuid::new_v4(), "resumable@example.com", 0, total_size);

    let create = CreateResumableUploadUseCase::new(
        repo.clone(),
        storage.clone(),
        ids,
        clock.clone(),
        total_size,
        900,
    );
    let created = create
        .execute(
            &owner,
            ResumableUploadRequest {
                filename: "video.txt".to_string(),
                parent_folder_id: None,
                content_type: "text/plain".to_string(),
                size_bytes: total_size,
                checksum_sha256: None,
                part_size_bytes: Some(part_size),
            },
        )
        .await
        .unwrap();
    assert_eq!(created.part_size_bytes, part_size);

    let presign = PresignUploadPartUseCase::new(repo.clone(), storage.clone(), clock.clone(), 900);
    let first_part = presign
        .execute(
            &owner,
            PresignUploadPartInput {
                file_id: created.file_id,
                part_number: 1,
            },
        )
        .await
        .unwrap();
    assert_eq!(first_part.expected_size_bytes, part_size);
    assert!(first_part.upload_url.contains("partNumber=1"));

    let record = RecordUploadPartUseCase::new(repo.clone(), clock.clone());
    record
        .execute(
            &owner,
            RecordUploadPartInput {
                file_id: created.file_id,
                part_number: 1,
                size_bytes: part_size,
                etag: "\"etag-1\"".to_string(),
            },
        )
        .await
        .unwrap();
    record
        .execute(
            &owner,
            RecordUploadPartInput {
                file_id: created.file_id,
                part_number: 2,
                size_bytes: part_size,
                etag: "\"etag-2\"".to_string(),
            },
        )
        .await
        .unwrap();
    record
        .execute(
            &owner,
            RecordUploadPartInput {
                file_id: created.file_id,
                part_number: 3,
                size_bytes: 2,
                etag: "\"etag-3\"".to_string(),
            },
        )
        .await
        .unwrap();

    let status = GetUploadStatusUseCase::new(repo.clone())
        .execute(&owner, created.file_id)
        .await
        .unwrap();
    assert_eq!(status.parts.len(), 3);

    storage.put_object(&created.object_key, total_size);
    let finalize = FinalizeResumableUploadUseCase::new(repo.clone(), storage.clone(), clock);
    let file = finalize.execute(&owner, created.file_id).await.unwrap();
    assert_eq!(file.state, FileState::Complete);
    assert_eq!(
        storage.multipart_completions()[0].2,
        vec![
            crate::application::ports::object_storage::CompletedUploadPart {
                part_number: 1,
                etag: "\"etag-1\"".to_string(),
            },
            crate::application::ports::object_storage::CompletedUploadPart {
                part_number: 2,
                etag: "\"etag-2\"".to_string(),
            },
            crate::application::ports::object_storage::CompletedUploadPart {
                part_number: 3,
                etag: "\"etag-3\"".to_string(),
            },
        ]
    );
}

#[tokio::test]
async fn resumable_upload_rejects_missing_or_wrong_sized_parts() {
    let repo = Arc::new(FakeFileRepository::default());
    let storage = Arc::new(FakeObjectStorage::default());
    let ids = Arc::new(SequenceIdGenerator::new([Uuid::new_v4()]));
    let clock = Arc::new(FixedClock::new(fixed_now()));
    let part_size = MIN_RESUMABLE_PART_SIZE_BYTES;
    let total_size = part_size * 2 + 2;
    let owner = user(Uuid::new_v4(), "parts@example.com", 0, total_size);
    let created = CreateResumableUploadUseCase::new(
        repo.clone(),
        storage.clone(),
        ids,
        clock.clone(),
        total_size,
        900,
    )
    .execute(
        &owner,
        ResumableUploadRequest {
            filename: "video.txt".to_string(),
            parent_folder_id: None,
            content_type: "text/plain".to_string(),
            size_bytes: total_size,
            checksum_sha256: None,
            part_size_bytes: Some(part_size),
        },
    )
    .await
    .unwrap();

    let record = RecordUploadPartUseCase::new(repo.clone(), clock.clone());
    assert_eq!(
        record
            .execute(
                &owner,
                RecordUploadPartInput {
                    file_id: created.file_id,
                    part_number: 2,
                    size_bytes: part_size - 1,
                    etag: "\"wrong\"".to_string(),
                },
            )
            .await
            .unwrap_err(),
        AppError::Domain(DomainError::InvalidFileState)
    );

    record
        .execute(
            &owner,
            RecordUploadPartInput {
                file_id: created.file_id,
                part_number: 1,
                size_bytes: part_size,
                etag: "\"etag-1\"".to_string(),
            },
        )
        .await
        .unwrap();
    storage.put_object(&created.object_key, total_size);
    assert_eq!(
        FinalizeResumableUploadUseCase::new(repo, storage, clock)
            .execute(&owner, created.file_id)
            .await
            .unwrap_err(),
        AppError::Domain(DomainError::InvalidFileState)
    );
}

#[tokio::test]
async fn resumable_session_ttl_uses_configured_lifetime_and_expires_at_boundary() {
    let repo = Arc::new(FakeFileRepository::default());
    let storage = Arc::new(FakeObjectStorage::default());
    let ids = Arc::new(SequenceIdGenerator::new([Uuid::new_v4()]));
    let created_at = fixed_now();
    let create_clock = Arc::new(FixedClock::new(created_at));
    let part_size = MIN_RESUMABLE_PART_SIZE_BYTES;
    let total_size = part_size;
    let owner = user(Uuid::new_v4(), "ttl@example.com", 0, total_size);
    let ttl_seconds = 86_400; // 24h decoupled from the 15 min presigned URL TTL

    let created = CreateResumableUploadUseCase::new(
        repo.clone(),
        storage.clone(),
        ids,
        create_clock,
        total_size,
        ttl_seconds,
    )
    .execute(
        &owner,
        ResumableUploadRequest {
            filename: "big.txt".to_string(),
            parent_folder_id: None,
            content_type: "text/plain".to_string(),
            size_bytes: total_size,
            checksum_sha256: None,
            part_size_bytes: Some(part_size),
        },
    )
    .await
    .unwrap();

    // The session lives for the full configured window, not the presigned URL TTL.
    assert_eq!(
        created.expires_at,
        created_at + chrono::Duration::seconds(ttl_seconds)
    );

    let presign_url_ttl = 900;
    let expires_at = created.expires_at;

    // One second before expiry the session is still usable.
    let before = Arc::new(FixedClock::new(expires_at - chrono::Duration::seconds(1)));
    PresignUploadPartUseCase::new(repo.clone(), storage.clone(), before, presign_url_ttl)
        .execute(
            &owner,
            PresignUploadPartInput {
                file_id: created.file_id,
                part_number: 1,
            },
        )
        .await
        .unwrap();

    // Exactly at the boundary the session is expired (upload_expires_at <= now).
    let at_boundary = Arc::new(FixedClock::new(expires_at));
    assert_eq!(
        PresignUploadPartUseCase::new(repo, storage, at_boundary, presign_url_ttl)
            .execute(
                &owner,
                PresignUploadPartInput {
                    file_id: created.file_id,
                    part_number: 1,
                },
            )
            .await
            .unwrap_err(),
        AppError::Domain(DomainError::InvalidFileState)
    );
}

#[tokio::test]
async fn list_pending_uploads_returns_active_sessions_with_part_counts() {
    let repo = Arc::new(FakeFileRepository::default());
    let storage = Arc::new(FakeObjectStorage::default());
    let ids = Arc::new(SequenceIdGenerator::new([Uuid::new_v4()]));
    let clock = Arc::new(FixedClock::new(fixed_now()));
    let part_size = MIN_RESUMABLE_PART_SIZE_BYTES;
    let total_size = part_size * 2 + 2;
    let owner = user(Uuid::new_v4(), "pending@example.com", 0, total_size);

    let created = CreateResumableUploadUseCase::new(
        repo.clone(),
        storage.clone(),
        ids,
        clock.clone(),
        total_size,
        86_400,
    )
    .execute(
        &owner,
        ResumableUploadRequest {
            filename: "movie.txt".to_string(),
            parent_folder_id: None,
            content_type: "text/plain".to_string(),
            size_bytes: total_size,
            checksum_sha256: None,
            part_size_bytes: Some(part_size),
        },
    )
    .await
    .unwrap();

    let list = ListPendingUploadsUseCase::new(repo.clone(), clock.clone());
    let before = list.execute(&owner).await.unwrap();
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].file_id, created.file_id);
    assert_eq!(before[0].parts_received, 0);
    assert_eq!(before[0].size_bytes, total_size);
    assert_eq!(before[0].part_size_bytes, part_size);

    RecordUploadPartUseCase::new(repo.clone(), clock.clone())
        .execute(
            &owner,
            RecordUploadPartInput {
                file_id: created.file_id,
                part_number: 1,
                size_bytes: part_size,
                etag: "\"etag-1\"".to_string(),
            },
        )
        .await
        .unwrap();

    let after_part = list.execute(&owner).await.unwrap();
    assert_eq!(after_part[0].parts_received, 1);

    // A different owner never sees these sessions.
    let stranger = user(Uuid::new_v4(), "stranger@example.com", 0, total_size);
    assert!(list.execute(&stranger).await.unwrap().is_empty());
}

#[tokio::test]
async fn list_and_download_only_return_completed_files() {
    let repo = Arc::new(FakeFileRepository::default());
    let file_id = Uuid::parse_str("55555555-5555-4555-8555-555555555555").unwrap();
    let storage = Arc::new(FakeObjectStorage::default());
    let owner = user(Uuid::new_v4(), "owner@example.com", 0, 100);
    repo.insert_completed(&owner, completed_file(file_id, "owner/object"));

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

#[tokio::test]
async fn delete_restore_and_trash_are_owner_scoped() {
    let repo = Arc::new(FakeFileRepository::default());
    let owner = user(
        Uuid::parse_str("66666666-6666-4666-8666-666666666666").unwrap(),
        "owner@example.com",
        0,
        100,
    );
    let other = user(
        Uuid::parse_str("77777777-7777-4777-8777-777777777777").unwrap(),
        "other@example.com",
        0,
        100,
    );
    let file_id = Uuid::parse_str("88888888-8888-4888-8888-888888888888").unwrap();
    repo.insert_completed(&owner, completed_file(file_id, "owner/deleted"));

    assert_eq!(
        DeleteFileUseCase::new(repo.clone())
            .execute(&other, file_id)
            .await
            .unwrap_err(),
        AppError::Domain(DomainError::FileNotFound)
    );

    DeleteFileUseCase::new(repo.clone())
        .execute(&owner, file_id)
        .await
        .unwrap();
    assert!(
        ListFilesUseCase::new(repo.clone())
            .execute(&owner)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        ListTrashUseCase::new(repo.clone())
            .execute(&owner)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        ListTrashUseCase::new(repo.clone())
            .execute(&other)
            .await
            .unwrap()
            .is_empty()
    );

    assert_eq!(
        RestoreFileUseCase::new(repo.clone())
            .execute(&other, file_id)
            .await
            .unwrap_err(),
        AppError::Domain(DomainError::FileNotFound)
    );
    let restored = RestoreFileUseCase::new(repo.clone())
        .execute(&owner, file_id)
        .await
        .unwrap();
    assert_eq!(restored.deleted_at, None);
}

#[tokio::test]
async fn deleted_files_cannot_be_downloaded_or_shared_until_restored() {
    let repo = Arc::new(FakeFileRepository::default());
    let storage = Arc::new(FakeObjectStorage::default());
    let auth_repo = Arc::new(FakeAuthRepository::default());
    let owner = user(
        Uuid::parse_str("99999999-9999-4999-8999-999999999999").unwrap(),
        "owner@example.com",
        0,
        100,
    );
    let grantee = user(
        Uuid::parse_str("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa").unwrap(),
        "friend@example.com",
        0,
        100,
    );
    auth_repo.add_user(grantee.clone(), "password123");
    repo.insert_completed(&owner, completed_file(grantee.id, "owner/shared"));
    repo.insert_user(&grantee);

    DeleteFileUseCase::new(repo.clone())
        .execute(&owner, grantee.id)
        .await
        .unwrap();

    assert_eq!(
        DownloadFileUseCase::new(repo.clone(), storage, 900)
            .execute(&owner, grantee.id)
            .await
            .unwrap_err(),
        AppError::Domain(DomainError::FileNotFound)
    );
    assert_eq!(
        ShareFileUseCase::new(repo.clone(), auth_repo)
            .execute(
                &owner,
                ShareFileInput {
                    file_id: grantee.id,
                    email: "friend@example.com".to_string(),
                },
            )
            .await
            .unwrap_err(),
        AppError::Domain(DomainError::FileNotFound)
    );
}

#[tokio::test]
async fn share_file_is_idempotent_and_rejects_self_or_unknown_email() {
    let repo = Arc::new(FakeFileRepository::default());
    let auth_repo = Arc::new(FakeAuthRepository::default());
    let owner = user(
        Uuid::parse_str("bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb").unwrap(),
        "owner@example.com",
        0,
        100,
    );
    let grantee = user(
        Uuid::parse_str("cccccccc-cccc-4ccc-8ccc-cccccccccccc").unwrap(),
        "friend@example.com",
        0,
        100,
    );
    auth_repo.add_user(owner.clone(), "password123");
    auth_repo.add_user(grantee.clone(), "password123");
    repo.insert_completed(&owner, completed_file(owner.id, "owner/shared"));
    repo.insert_user(&grantee);
    let use_case = ShareFileUseCase::new(repo.clone(), auth_repo.clone());

    let share = use_case
        .execute(
            &owner,
            ShareFileInput {
                file_id: owner.id,
                email: " FRIEND@example.com ".to_string(),
            },
        )
        .await
        .unwrap();
    let duplicate = use_case
        .execute(
            &owner,
            ShareFileInput {
                file_id: owner.id,
                email: "friend@example.com".to_string(),
            },
        )
        .await
        .unwrap();
    assert_eq!(share.grantee.id, grantee.id);
    assert_eq!(duplicate.grantee.id, grantee.id);
    assert_eq!(repo.shares.lock().unwrap().len(), 1);

    assert_eq!(
        use_case
            .execute(
                &owner,
                ShareFileInput {
                    file_id: owner.id,
                    email: "owner@example.com".to_string(),
                },
            )
            .await
            .unwrap_err(),
        AppError::Domain(DomainError::Validation("Cannot share a file with yourself"))
    );
    assert_eq!(
        use_case
            .execute(
                &owner,
                ShareFileInput {
                    file_id: owner.id,
                    email: "missing@example.com".to_string(),
                },
            )
            .await
            .unwrap_err(),
        AppError::Domain(DomainError::UserNotFound)
    );
}

#[tokio::test]
async fn share_lists_revoke_and_shared_with_me_are_scoped() {
    let repo = Arc::new(FakeFileRepository::default());
    let owner = user(
        Uuid::parse_str("dddddddd-dddd-4ddd-8ddd-dddddddddddd").unwrap(),
        "owner@example.com",
        0,
        100,
    );
    let grantee = user(
        Uuid::parse_str("eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee").unwrap(),
        "friend@example.com",
        0,
        100,
    );
    let other = user(
        Uuid::parse_str("ffffffff-ffff-4fff-8fff-ffffffffffff").unwrap(),
        "other@example.com",
        0,
        100,
    );
    let file_id = Uuid::parse_str("12121212-1212-4212-8212-121212121212").unwrap();
    repo.insert_completed(&owner, completed_file(file_id, "owner/shared"));
    repo.insert_user(&grantee);
    repo.shares
        .lock()
        .unwrap()
        .insert((file_id, grantee.id), fixed_now());

    let shares = ListSharesUseCase::new(repo.clone())
        .execute(&owner, file_id)
        .await
        .unwrap();
    assert_eq!(shares.len(), 1);
    assert_eq!(shares[0].grantee.id, grantee.id);
    assert_eq!(
        ListSharesUseCase::new(repo.clone())
            .execute(&other, file_id)
            .await
            .unwrap_err(),
        AppError::Domain(DomainError::FileNotFound)
    );

    let shared = ListSharedWithMeUseCase::new(repo.clone())
        .execute(&grantee)
        .await
        .unwrap();
    assert_eq!(shared.len(), 1);
    assert_eq!(shared[0].owner.id, owner.id);
    assert!(
        ListSharedWithMeUseCase::new(repo.clone())
            .execute(&other)
            .await
            .unwrap()
            .is_empty()
    );

    assert_eq!(
        RevokeShareUseCase::new(repo.clone())
            .execute(&other, file_id, grantee.id)
            .await
            .unwrap_err(),
        AppError::Domain(DomainError::FileNotFound)
    );
    RevokeShareUseCase::new(repo.clone())
        .execute(&owner, file_id, grantee.id)
        .await
        .unwrap();
    assert!(
        ListSharedWithMeUseCase::new(repo.clone())
            .execute(&grantee)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn shared_grantee_can_download_until_share_is_revoked_or_file_deleted() {
    let repo = Arc::new(FakeFileRepository::default());
    let storage = Arc::new(FakeObjectStorage::default());
    let owner = user(
        Uuid::parse_str("13131313-1313-4313-8313-131313131313").unwrap(),
        "owner@example.com",
        0,
        100,
    );
    let grantee = user(
        Uuid::parse_str("14141414-1414-4414-8414-141414141414").unwrap(),
        "friend@example.com",
        0,
        100,
    );
    let file_id = Uuid::parse_str("15151515-1515-4515-8515-151515151515").unwrap();
    repo.insert_completed(&owner, completed_file(file_id, "owner/download"));
    repo.insert_user(&grantee);
    repo.shares
        .lock()
        .unwrap()
        .insert((file_id, grantee.id), fixed_now());

    let download = DownloadFileUseCase::new(repo.clone(), storage.clone(), 900)
        .execute(&grantee, file_id)
        .await
        .unwrap();
    assert!(download.download_url.contains("owner/download"));

    RevokeShareUseCase::new(repo.clone())
        .execute(&owner, file_id, grantee.id)
        .await
        .unwrap();
    assert_eq!(
        DownloadFileUseCase::new(repo.clone(), storage.clone(), 900)
            .execute(&grantee, file_id)
            .await
            .unwrap_err(),
        AppError::Domain(DomainError::FileNotFound)
    );

    repo.shares
        .lock()
        .unwrap()
        .insert((file_id, grantee.id), fixed_now());
    DeleteFileUseCase::new(repo.clone())
        .execute(&owner, file_id)
        .await
        .unwrap();
    assert_eq!(
        DownloadFileUseCase::new(repo, storage, 900)
            .execute(&grantee, file_id)
            .await
            .unwrap_err(),
        AppError::Domain(DomainError::FileNotFound)
    );
}

#[tokio::test]
async fn search_files_is_case_insensitive_acl_scoped_and_excludes_trash_by_default() {
    let repo = Arc::new(FakeFileRepository::default());
    let owner = user(
        Uuid::parse_str("23232323-2323-4323-8323-232323232323").unwrap(),
        "owner@example.com",
        0,
        100,
    );
    let grantee = user(
        Uuid::parse_str("24242424-2424-4424-8424-242424242424").unwrap(),
        "friend@example.com",
        0,
        100,
    );
    let other = user(
        Uuid::parse_str("25252525-2525-4525-8525-252525252525").unwrap(),
        "other@example.com",
        0,
        100,
    );

    let owned_id = Uuid::parse_str("26262626-2626-4626-8626-262626262626").unwrap();
    let shared_id = Uuid::parse_str("27272727-2727-4727-8727-272727272727").unwrap();
    let private_id = Uuid::parse_str("28282828-2828-4828-8828-282828282828").unwrap();
    let pending_id = Uuid::parse_str("29292929-2929-4929-8929-292929292929").unwrap();
    let deleted_id = Uuid::parse_str("30303030-3030-4030-8030-303030303030").unwrap();
    let deleted_shared_id = Uuid::parse_str("31313131-3131-4131-8131-313131313131").unwrap();

    let mut owned = completed_file(owned_id, "friend/quarterly");
    owned.filename = "Report-Quarterly.txt".to_string();
    repo.insert_completed(&grantee, owned);

    let mut shared = completed_file(shared_id, "owner/shared");
    shared.filename = "shared-report.txt".to_string();
    repo.insert_completed(&owner, shared);
    repo.shares
        .lock()
        .unwrap()
        .insert((shared_id, grantee.id), fixed_now());

    let mut private = completed_file(private_id, "other/private");
    private.filename = "private-report.txt".to_string();
    repo.insert_completed(&other, private);

    let mut pending = completed_file(pending_id, "friend/pending");
    pending.filename = "pending-report.txt".to_string();
    pending.state = FileState::Pending;
    repo.insert_completed(&grantee, pending);

    let mut deleted = completed_file(deleted_id, "friend/deleted");
    deleted.filename = "deleted-report.txt".to_string();
    deleted.deleted_at = Some(fixed_now());
    repo.insert_completed(&grantee, deleted);

    let mut deleted_shared = completed_file(deleted_shared_id, "owner/deleted-shared");
    deleted_shared.filename = "deleted-shared-report.txt".to_string();
    deleted_shared.deleted_at = Some(fixed_now());
    repo.insert_completed(&owner, deleted_shared);
    repo.shares
        .lock()
        .unwrap()
        .insert((deleted_shared_id, grantee.id), fixed_now());

    let use_case = SearchFilesUseCase::new(repo.clone());
    let results = use_case
        .execute(
            &grantee,
            SearchFilesInput {
                query: "REPORT".to_string(),
                include_deleted: false,
                limit: None,
            },
        )
        .await
        .unwrap();

    let filenames: Vec<_> = results
        .iter()
        .map(|result| result.file.filename.as_str())
        .collect();
    assert_eq!(filenames, vec!["Report-Quarterly.txt", "shared-report.txt"]);
    assert_eq!(results[0].access, SearchAccess::Owned);
    assert_eq!(results[1].access, SearchAccess::Shared);
    assert_eq!(results[1].owner.as_ref().unwrap().id, owner.id);

    let with_deleted = SearchFilesUseCase::new(repo.clone())
        .execute(
            &grantee,
            SearchFilesInput {
                query: "deleted".to_string(),
                include_deleted: true,
                limit: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(with_deleted.len(), 1);
    assert_eq!(with_deleted[0].file.id, deleted_id);

    assert_eq!(
        SearchFilesUseCase::new(repo.clone())
            .execute(
                &grantee,
                SearchFilesInput {
                    query: " ".to_string(),
                    include_deleted: false,
                    limit: None,
                },
            )
            .await
            .unwrap_err(),
        AppError::Domain(DomainError::Validation("Enter a search query"))
    );

    let mut literal = completed_file(Uuid::new_v4(), "friend/literal");
    literal.filename = "literal_%_report.txt".to_string();
    repo.insert_completed(&grantee, literal);
    let mut similar = completed_file(Uuid::new_v4(), "friend/similar");
    similar.filename = "literal-x-report.txt".to_string();
    repo.insert_completed(&grantee, similar);

    let wildcard_results = SearchFilesUseCase::new(repo)
        .execute(
            &grantee,
            SearchFilesInput {
                query: "%_".to_string(),
                include_deleted: false,
                limit: Some(10),
            },
        )
        .await
        .unwrap();
    assert_eq!(wildcard_results.len(), 1);
    assert_eq!(wildcard_results[0].file.filename, "literal_%_report.txt");
}

#[tokio::test]
async fn folder_create_browse_rename_move_and_cycle_rules_are_owner_scoped() {
    let repo = Arc::new(FakeFileRepository::default());
    let owner = user(
        Uuid::parse_str("16161616-1616-4616-8616-161616161616").unwrap(),
        "owner@example.com",
        0,
        100,
    );
    let other = user(
        Uuid::parse_str("17171717-1717-4717-8717-171717171717").unwrap(),
        "other@example.com",
        0,
        100,
    );
    let root_id = Uuid::parse_str("18181818-1818-4818-8818-181818181818").unwrap();
    let child_id = Uuid::parse_str("19191919-1919-4919-8919-191919191919").unwrap();

    *repo.next_folder_id.lock().unwrap() = Some(root_id);
    let root = CreateFolderUseCase::new(repo.clone())
        .execute(
            &owner,
            CreateFolderInput {
                name: "Projects".to_string(),
                parent_folder_id: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(root.name, "Projects");

    *repo.next_folder_id.lock().unwrap() = Some(child_id);
    CreateFolderUseCase::new(repo.clone())
        .execute(
            &owner,
            CreateFolderInput {
                name: "Client".to_string(),
                parent_folder_id: Some(root_id),
            },
        )
        .await
        .unwrap();

    let browse = BrowseFolderUseCase::new(repo.clone())
        .execute(&owner, Some(root_id))
        .await
        .unwrap();
    assert_eq!(browse.folders[0].id, child_id);
    assert_eq!(
        ListFoldersUseCase::new(repo.clone())
            .execute(&owner)
            .await
            .unwrap()
            .len(),
        2
    );

    let renamed = UpdateFolderUseCase::new(repo.clone())
        .execute(
            &owner,
            UpdateFolderInput {
                folder_id: root_id,
                name: Some("Work".to_string()),
                parent_folder_id: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(renamed.name, "Work");

    assert_eq!(
        UpdateFolderUseCase::new(repo.clone())
            .execute(
                &owner,
                UpdateFolderInput {
                    folder_id: root_id,
                    name: None,
                    parent_folder_id: Some(Some(child_id)),
                },
            )
            .await
            .unwrap_err(),
        AppError::Domain(DomainError::InvalidFileState)
    );

    assert_eq!(
        BrowseFolderUseCase::new(repo.clone())
            .execute(&other, Some(root_id))
            .await
            .unwrap_err(),
        AppError::Domain(DomainError::FileNotFound)
    );
    assert_eq!(
        CreateFolderUseCase::new(repo)
            .execute(
                &other,
                CreateFolderInput {
                    name: "Invalid".to_string(),
                    parent_folder_id: Some(root_id),
                },
            )
            .await
            .unwrap_err(),
        AppError::Domain(DomainError::FileNotFound)
    );
}

#[tokio::test]
async fn folder_delete_and_restore_hide_and_restore_descendant_files() {
    let repo = Arc::new(FakeFileRepository::default());
    let owner = user(
        Uuid::parse_str("20202020-2020-4020-8020-202020202020").unwrap(),
        "owner@example.com",
        0,
        100,
    );
    let folder_id = Uuid::parse_str("21212121-2121-4121-8121-212121212121").unwrap();
    let file_id = Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap();
    repo.insert_folder(
        &owner,
        Folder {
            id: folder_id,
            name: "Work".to_string(),
            parent_folder_id: None,
            created_at: fixed_now(),
            updated_at: fixed_now(),
            deleted_at: None,
        },
    );
    let mut file = completed_file(file_id, "owner/work");
    file.parent_folder_id = Some(folder_id);
    repo.insert_completed(&owner, file);

    DeleteFolderUseCase::new(repo.clone())
        .execute(&owner, folder_id)
        .await
        .unwrap();
    assert!(
        BrowseFolderUseCase::new(repo.clone())
            .execute(&owner, Some(folder_id))
            .await
            .is_err()
    );
    assert_eq!(
        ListDriveTrashUseCase::new(repo.clone())
            .execute(&owner)
            .await
            .unwrap()
            .folders
            .len(),
        1
    );

    RestoreFolderUseCase::new(repo.clone())
        .execute(&owner, folder_id)
        .await
        .unwrap();
    let browse = BrowseFolderUseCase::new(repo.clone())
        .execute(&owner, Some(folder_id))
        .await
        .unwrap();
    assert_eq!(browse.files[0].id, file_id);

    let moved = UpdateFileUseCase::new(repo)
        .execute(
            &owner,
            UpdateFileInput {
                file_id,
                filename: Some("renamed.txt".to_string()),
                parent_folder_id: Some(None),
            },
        )
        .await
        .unwrap();
    assert_eq!(moved.filename, "renamed.txt");
    assert_eq!(moved.parent_folder_id, None);
}
