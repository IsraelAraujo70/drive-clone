use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::application::AppError;
use crate::application::ports::clock::Clock;
use crate::application::ports::files::{
    CreateResumableUploadRecord, FileRepository, RecordUploadPartRecord,
};
use crate::application::ports::id_generator::IdGenerator;
use crate::application::ports::object_storage::{CompletedUploadPart, ObjectStorage};
use crate::domain::auth::User;
use crate::domain::error::DomainError;
use crate::domain::files::{
    FileState, ResumableUploadRequest, ResumableUploadSession, UploadPart, ensure_quota,
    validate_checksum, validate_content_type, validate_filename, validate_size,
};

pub const DEFAULT_RESUMABLE_PART_SIZE_BYTES: i64 = 8 * 1024 * 1024;
pub const MIN_RESUMABLE_PART_SIZE_BYTES: i64 = 5 * 1024 * 1024;
const MAX_MULTIPART_PARTS: i64 = 10_000;

#[derive(Debug, Clone)]
pub struct CreateResumableUploadOutput {
    pub file_id: Uuid,
    pub object_key: String,
    pub part_size_bytes: i64,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PresignUploadPartInput {
    pub file_id: Uuid,
    pub part_number: i32,
}

#[derive(Debug, Clone)]
pub struct PresignUploadPartOutput {
    pub file_id: Uuid,
    pub part_number: i32,
    pub upload_url: String,
    pub expires_at: DateTime<Utc>,
    pub expected_size_bytes: i64,
}

#[derive(Debug, Clone)]
pub struct RecordUploadPartInput {
    pub file_id: Uuid,
    pub part_number: i32,
    pub size_bytes: i64,
    pub etag: String,
}

#[derive(Debug, Clone)]
pub struct ExpireUploadsOutput {
    pub expired_count: usize,
    pub aborted_count: usize,
}

#[derive(Clone)]
pub struct CreateResumableUploadUseCase {
    file_repository: Arc<dyn FileRepository>,
    storage: Arc<dyn ObjectStorage>,
    id_generator: Arc<dyn IdGenerator>,
    clock: Arc<dyn Clock>,
    max_file_size_bytes: i64,
    upload_ttl_seconds: i64,
}

impl CreateResumableUploadUseCase {
    pub fn new(
        file_repository: Arc<dyn FileRepository>,
        storage: Arc<dyn ObjectStorage>,
        id_generator: Arc<dyn IdGenerator>,
        clock: Arc<dyn Clock>,
        max_file_size_bytes: i64,
        upload_ttl_seconds: i64,
    ) -> Self {
        Self {
            file_repository,
            storage,
            id_generator,
            clock,
            max_file_size_bytes,
            upload_ttl_seconds,
        }
    }

    pub async fn execute(
        &self,
        user: &User,
        request: ResumableUploadRequest,
    ) -> Result<CreateResumableUploadOutput, AppError> {
        let filename = validate_filename(&request.filename)?;
        let content_type = validate_content_type(&request.content_type)?;
        validate_size(request.size_bytes, self.max_file_size_bytes)?;
        let checksum_sha256 = validate_checksum(request.checksum_sha256.as_deref())?;
        let part_size_bytes = normalized_part_size(request.size_bytes, request.part_size_bytes)?;

        let pending_cap = user.storage_quota_bytes.max(0);
        let pending_bytes = self
            .file_repository
            .pending_bytes_for_owner(user.id, pending_cap)
            .await?;
        ensure_quota(
            user.storage_used_bytes,
            user.storage_quota_bytes,
            pending_bytes,
            request.size_bytes,
        )?;

        let object_key = format!("{}/{}", user.id, self.id_generator.new_uuid());
        let multipart_upload_id = self
            .storage
            .create_multipart_upload(&object_key, &content_type)
            .await?;
        let expires_at = self.clock.now() + Duration::seconds(self.upload_ttl_seconds);

        let session = match self
            .file_repository
            .create_resumable_upload(CreateResumableUploadRecord {
                owner_id: user.id,
                filename,
                parent_folder_id: request.parent_folder_id,
                content_type,
                size_bytes: request.size_bytes,
                checksum_sha256,
                object_key: object_key.clone(),
                multipart_upload_id: multipart_upload_id.clone(),
                part_size_bytes,
                upload_expires_at: expires_at,
            })
            .await
        {
            Ok(session) => session,
            Err(error) => {
                let _ = self
                    .storage
                    .abort_multipart_upload(&object_key, &multipart_upload_id)
                    .await;
                return Err(error.into());
            }
        };

        Ok(CreateResumableUploadOutput {
            file_id: session.file_id,
            object_key: session.object_key,
            part_size_bytes: session.part_size_bytes,
            expires_at: session.upload_expires_at,
        })
    }
}

#[derive(Clone)]
pub struct GetUploadStatusUseCase {
    file_repository: Arc<dyn FileRepository>,
}

impl GetUploadStatusUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>) -> Self {
        Self { file_repository }
    }

    pub async fn execute(
        &self,
        user: &User,
        file_id: Uuid,
    ) -> Result<ResumableUploadSession, AppError> {
        self.file_repository
            .find_resumable_upload(user.id, file_id)
            .await?
            .ok_or_else(|| DomainError::FileNotFound.into())
    }
}

#[derive(Clone)]
pub struct PresignUploadPartUseCase {
    file_repository: Arc<dyn FileRepository>,
    storage: Arc<dyn ObjectStorage>,
    clock: Arc<dyn Clock>,
    presigned_url_ttl_seconds: i64,
}

impl PresignUploadPartUseCase {
    pub fn new(
        file_repository: Arc<dyn FileRepository>,
        storage: Arc<dyn ObjectStorage>,
        clock: Arc<dyn Clock>,
        presigned_url_ttl_seconds: i64,
    ) -> Self {
        Self {
            file_repository,
            storage,
            clock,
            presigned_url_ttl_seconds,
        }
    }

    pub async fn execute(
        &self,
        user: &User,
        input: PresignUploadPartInput,
    ) -> Result<PresignUploadPartOutput, AppError> {
        let session = pending_session(
            self.file_repository.as_ref(),
            user.id,
            input.file_id,
            self.clock.now(),
        )
        .await?;
        let expected_size_bytes = expected_part_size(&session, input.part_number)?;
        let presigned = self
            .storage
            .presign_upload_part(
                &session.object_key,
                &session.multipart_upload_id,
                input.part_number,
                self.presigned_url_ttl_seconds,
            )
            .await?;

        Ok(PresignUploadPartOutput {
            file_id: session.file_id,
            part_number: input.part_number,
            upload_url: presigned.url,
            expires_at: presigned.expires_at,
            expected_size_bytes,
        })
    }
}

#[derive(Clone)]
pub struct RecordUploadPartUseCase {
    file_repository: Arc<dyn FileRepository>,
    clock: Arc<dyn Clock>,
}

impl RecordUploadPartUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            file_repository,
            clock,
        }
    }

    pub async fn execute(
        &self,
        user: &User,
        input: RecordUploadPartInput,
    ) -> Result<UploadPart, AppError> {
        let session = pending_session(
            self.file_repository.as_ref(),
            user.id,
            input.file_id,
            self.clock.now(),
        )
        .await?;
        let expected_size = expected_part_size(&session, input.part_number)?;
        if input.size_bytes != expected_size {
            return Err(DomainError::InvalidFileState.into());
        }
        let etag = input.etag.trim();
        if etag.is_empty() {
            return Err(DomainError::Validation("etag is required").into());
        }

        self.file_repository
            .record_upload_part(RecordUploadPartRecord {
                owner_id: user.id,
                file_id: input.file_id,
                part_number: input.part_number,
                size_bytes: input.size_bytes,
                etag: etag.to_string(),
            })
            .await
            .map_err(AppError::from)
    }
}

#[derive(Clone)]
pub struct FinalizeResumableUploadUseCase {
    file_repository: Arc<dyn FileRepository>,
    storage: Arc<dyn ObjectStorage>,
    clock: Arc<dyn Clock>,
}

impl FinalizeResumableUploadUseCase {
    pub fn new(
        file_repository: Arc<dyn FileRepository>,
        storage: Arc<dyn ObjectStorage>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            file_repository,
            storage,
            clock,
        }
    }

    pub async fn execute(
        &self,
        user: &User,
        file_id: Uuid,
    ) -> Result<crate::domain::files::DriveFile, AppError> {
        let session = pending_session(
            self.file_repository.as_ref(),
            user.id,
            file_id,
            self.clock.now(),
        )
        .await?;
        validate_complete_parts(&session)?;
        let completed_parts = session
            .parts
            .iter()
            .map(|part| CompletedUploadPart {
                part_number: part.part_number,
                etag: part.etag.clone(),
            })
            .collect::<Vec<_>>();

        self.storage
            .complete_multipart_upload(
                &session.object_key,
                &session.multipart_upload_id,
                &completed_parts,
            )
            .await?;
        let object = self.storage.head_object(&session.object_key).await?;
        if object.content_length != session.size_bytes {
            return Err(AppError::Storage);
        }

        self.file_repository
            .complete_resumable_upload_once(user.id, file_id, session.size_bytes)
            .await
            .map_err(AppError::from)
    }
}

#[derive(Clone)]
pub struct ExpireResumableUploadsUseCase {
    file_repository: Arc<dyn FileRepository>,
    storage: Arc<dyn ObjectStorage>,
    clock: Arc<dyn Clock>,
}

impl ExpireResumableUploadsUseCase {
    pub fn new(
        file_repository: Arc<dyn FileRepository>,
        storage: Arc<dyn ObjectStorage>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            file_repository,
            storage,
            clock,
        }
    }

    pub async fn execute(&self, limit: i64) -> Result<ExpireUploadsOutput, AppError> {
        let expired = self
            .file_repository
            .expire_resumable_uploads(self.clock.now(), limit.clamp(1, 1000))
            .await?;
        let expired_count = expired.len();
        let mut aborted_count = 0;
        for upload in expired {
            let _file_id = upload.file_id;
            if self
                .storage
                .abort_multipart_upload(&upload.object_key, &upload.multipart_upload_id)
                .await
                .is_ok()
            {
                aborted_count += 1;
            }
        }
        Ok(ExpireUploadsOutput {
            expired_count,
            aborted_count,
        })
    }
}

async fn pending_session(
    repository: &dyn FileRepository,
    owner_id: Uuid,
    file_id: Uuid,
    now: DateTime<Utc>,
) -> Result<ResumableUploadSession, AppError> {
    let session = repository
        .find_resumable_upload(owner_id, file_id)
        .await?
        .ok_or(DomainError::FileNotFound)?;
    if session.state != FileState::Pending || session.upload_expires_at <= now {
        return Err(DomainError::InvalidFileState.into());
    }
    Ok(session)
}

fn normalized_part_size(size_bytes: i64, requested: Option<i64>) -> Result<i64, DomainError> {
    let minimum = ((size_bytes + MAX_MULTIPART_PARTS - 1) / MAX_MULTIPART_PARTS)
        .max(MIN_RESUMABLE_PART_SIZE_BYTES);
    let part_size = requested
        .unwrap_or(DEFAULT_RESUMABLE_PART_SIZE_BYTES)
        .max(minimum);
    if part_size <= 0 || part_count(size_bytes, part_size) > MAX_MULTIPART_PARTS {
        return Err(DomainError::InvalidFileState);
    }
    Ok(part_size)
}

fn part_count(size_bytes: i64, part_size_bytes: i64) -> i64 {
    (size_bytes + part_size_bytes - 1) / part_size_bytes
}

fn expected_part_size(
    session: &ResumableUploadSession,
    part_number: i32,
) -> Result<i64, DomainError> {
    if part_number <= 0 {
        return Err(DomainError::InvalidFileState);
    }
    let part_number = i64::from(part_number);
    let total_parts = part_count(session.size_bytes, session.part_size_bytes);
    if part_number > total_parts {
        return Err(DomainError::InvalidFileState);
    }
    let before = (part_number - 1) * session.part_size_bytes;
    Ok((session.size_bytes - before).min(session.part_size_bytes))
}

fn validate_complete_parts(session: &ResumableUploadSession) -> Result<(), DomainError> {
    let total_parts = part_count(session.size_bytes, session.part_size_bytes);
    if session.parts.len() != total_parts as usize {
        return Err(DomainError::InvalidFileState);
    }
    let mut sum = 0;
    for expected_part_number in 1..=total_parts {
        let part = session
            .parts
            .get((expected_part_number - 1) as usize)
            .ok_or(DomainError::InvalidFileState)?;
        if i64::from(part.part_number) != expected_part_number {
            return Err(DomainError::InvalidFileState);
        }
        let expected_size = expected_part_size(session, part.part_number)?;
        if part.size_bytes != expected_size {
            return Err(DomainError::InvalidFileState);
        }
        sum += part.size_bytes;
    }
    if sum != session.size_bytes {
        return Err(DomainError::InvalidFileState);
    }
    Ok(())
}
