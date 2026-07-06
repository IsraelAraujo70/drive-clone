use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::AppError;
use crate::application::ports::files::{CreatePendingFileRecord, FileRepository};
use crate::application::ports::id_generator::IdGenerator;
use crate::application::ports::object_storage::ObjectStorage;
use crate::domain::auth::User;
use crate::domain::files::{
    UploadRequest, ensure_quota, validate_checksum, validate_content_type, validate_filename,
    validate_size,
};

#[derive(Debug, Clone)]
pub struct CreateUploadOutput {
    pub file_id: Uuid,
    pub upload_url: String,
    pub object_key: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct CreateUploadUseCase {
    file_repository: Arc<dyn FileRepository>,
    storage: Arc<dyn ObjectStorage>,
    id_generator: Arc<dyn IdGenerator>,
    max_file_size_bytes: i64,
    presigned_url_ttl_seconds: i64,
}

impl CreateUploadUseCase {
    pub fn new(
        file_repository: Arc<dyn FileRepository>,
        storage: Arc<dyn ObjectStorage>,
        id_generator: Arc<dyn IdGenerator>,
        max_file_size_bytes: i64,
        presigned_url_ttl_seconds: i64,
    ) -> Self {
        Self {
            file_repository,
            storage,
            id_generator,
            max_file_size_bytes,
            presigned_url_ttl_seconds,
        }
    }

    pub async fn execute(
        &self,
        user: &User,
        request: UploadRequest,
    ) -> Result<CreateUploadOutput, AppError> {
        let filename = validate_filename(&request.filename)?;
        let content_type = validate_content_type(&request.content_type)?;
        validate_size(request.size_bytes, self.max_file_size_bytes)?;
        let checksum_sha256 = validate_checksum(request.checksum_sha256.as_deref())?;

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
        let presigned = self
            .storage
            .presign_put(
                &object_key,
                &content_type,
                request.size_bytes,
                self.presigned_url_ttl_seconds,
            )
            .await?;

        let pending = self
            .file_repository
            .create_pending_file(CreatePendingFileRecord {
                owner_id: user.id,
                filename,
                parent_folder_id: request.parent_folder_id,
                content_type,
                size_bytes: request.size_bytes,
                checksum_sha256,
                object_key,
            })
            .await?;

        Ok(CreateUploadOutput {
            file_id: pending.id,
            upload_url: presigned.url,
            object_key: pending.object_key,
            expires_at: presigned.expires_at,
        })
    }
}
