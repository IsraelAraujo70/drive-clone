use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::adapters::postgres::tx::map_sqlx_error;
use crate::application::ports::RepositoryError;
use crate::application::ports::files::{
    CreateFolderRecord, CreatePendingFileRecord, CreateResumableUploadRecord, ExpiredUploadRecord,
    FileRepository, RecordUploadPartRecord, SearchFilesRecord, UpdateFileRecord,
    UpdateFolderRecord,
};
use crate::domain::files::{
    ChangeEntityType, ChangeLogEntry, ChangeOp, DriveBrowse, DriveFile, FileShare, FileState,
    FileUser, Folder, FolderPathEntry, PendingFile, ResumableUploadSession, SearchAccess,
    SearchFileResult, SharedFile, UploadPart,
};

#[derive(Debug, Clone)]
pub struct PostgresFileRepository {
    pool: PgPool,
}

impl PostgresFileRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn escape_like_literal(value: &str) -> String {
        let mut escaped = String::with_capacity(value.len());
        for ch in value.chars() {
            match ch {
                '\\' | '%' | '_' => {
                    escaped.push('\\');
                    escaped.push(ch);
                }
                _ => escaped.push(ch),
            }
        }
        escaped
    }

    async fn ensure_active_parent(
        executor: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        owner_id: Uuid,
        parent_folder_id: Option<Uuid>,
    ) -> Result<(), RepositoryError> {
        let Some(parent_folder_id) = parent_folder_id else {
            return Ok(());
        };

        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM folders
                WHERE id = $1 AND owner_id = $2 AND deleted_at IS NULL
            )",
        )
        .bind(parent_folder_id)
        .bind(owner_id)
        .fetch_one(&mut **executor)
        .await
        .map_err(map_sqlx_error)?;

        if exists {
            Ok(())
        } else {
            Err(RepositoryError::NotFound)
        }
    }

    async fn breadcrumbs(
        &self,
        owner_id: Uuid,
        parent_folder_id: Option<Uuid>,
    ) -> Result<Vec<FolderPathEntry>, RepositoryError> {
        let Some(parent_folder_id) = parent_folder_id else {
            return Ok(Vec::new());
        };

        let mut rows = sqlx::query_as::<_, FolderPathRow>(
            "WITH RECURSIVE path AS (
                SELECT id, name, parent_folder_id, 0 AS depth
                FROM folders
                WHERE id = $1 AND owner_id = $2 AND deleted_at IS NULL
                UNION ALL
                SELECT f.id, f.name, f.parent_folder_id, path.depth + 1
                FROM folders f
                JOIN path ON path.parent_folder_id = f.id
                WHERE f.owner_id = $2 AND f.deleted_at IS NULL
             )
             SELECT id, name FROM path ORDER BY depth DESC",
        )
        .bind(parent_folder_id)
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        if rows.is_empty() {
            return Err(RepositoryError::NotFound);
        }

        Ok(rows
            .drain(..)
            .map(|row| FolderPathEntry {
                id: row.id,
                name: row.name,
            })
            .collect())
    }

    /// Records a change-feed entry inside an existing transaction. Bumps the
    /// owner's `change_seq` under a row lock (serializing the owner's writes so
    /// visibility order matches seq order) and inserts the tombstone/upsert
    /// marker. Must run in the same transaction as the mutation it describes.
    async fn record_change(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        owner_id: Uuid,
        entity_type: &str,
        entity_id: Uuid,
        op: &str,
    ) -> Result<i64, RepositoryError> {
        let seq: i64 = sqlx::query_scalar(
            "UPDATE users SET change_seq = change_seq + 1 WHERE id = $1 RETURNING change_seq",
        )
        .bind(owner_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_sqlx_error)?;

        sqlx::query(
            "INSERT INTO change_log (owner_id, seq, entity_type, entity_id, op)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(owner_id)
        .bind(seq)
        .bind(entity_type)
        .bind(entity_id)
        .bind(op)
        .execute(&mut **tx)
        .await
        .map_err(map_sqlx_error)?;

        Ok(seq)
    }
}

#[derive(Debug, sqlx::FromRow)]
struct UploadPartRow {
    part_number: i32,
    size_bytes: i64,
    etag: String,
}

impl From<UploadPartRow> for UploadPart {
    fn from(row: UploadPartRow) -> Self {
        Self {
            part_number: row.part_number,
            size_bytes: row.size_bytes,
            etag: row.etag,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ResumableUploadRow {
    id: Uuid,
    owner_id: Uuid,
    filename: String,
    parent_folder_id: Option<Uuid>,
    content_type: String,
    size_bytes: i64,
    checksum_sha256: Option<String>,
    object_key: String,
    state: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    multipart_upload_id: String,
    upload_expires_at: DateTime<Utc>,
    part_size_bytes: i64,
}

impl ResumableUploadRow {
    fn into_session(self, parts: Vec<UploadPart>) -> ResumableUploadSession {
        ResumableUploadSession {
            file_id: self.id,
            owner_id: self.owner_id,
            filename: self.filename,
            parent_folder_id: self.parent_folder_id,
            content_type: self.content_type,
            size_bytes: self.size_bytes,
            checksum_sha256: self.checksum_sha256,
            object_key: self.object_key,
            multipart_upload_id: self.multipart_upload_id,
            part_size_bytes: self.part_size_bytes,
            state: self.state.into(),
            upload_expires_at: self.upload_expires_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
            completed_at: self.completed_at,
            parts,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct PendingFileRow {
    id: Uuid,
    size_bytes: i64,
    object_key: String,
    state: String,
}

impl From<PendingFileRow> for PendingFile {
    fn from(row: PendingFileRow) -> Self {
        Self {
            id: row.id,
            size_bytes: row.size_bytes,
            object_key: row.object_key,
            state: row.state.into(),
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct DriveFileRow {
    id: Uuid,
    filename: String,
    parent_folder_id: Option<Uuid>,
    content_type: String,
    size_bytes: i64,
    checksum_sha256: Option<String>,
    object_key: String,
    state: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
}

impl From<DriveFileRow> for DriveFile {
    fn from(row: DriveFileRow) -> Self {
        Self {
            id: row.id,
            filename: row.filename,
            parent_folder_id: row.parent_folder_id,
            content_type: row.content_type,
            size_bytes: row.size_bytes,
            checksum_sha256: row.checksum_sha256,
            object_key: row.object_key,
            state: row.state.into(),
            created_at: row.created_at,
            updated_at: row.updated_at,
            completed_at: row.completed_at,
            deleted_at: row.deleted_at,
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct FolderRow {
    id: Uuid,
    name: String,
    parent_folder_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}

impl From<FolderRow> for Folder {
    fn from(row: FolderRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            parent_folder_id: row.parent_folder_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct FolderPathRow {
    id: Uuid,
    name: String,
}

#[derive(Debug, sqlx::FromRow)]
struct FileShareRow {
    file_id: Uuid,
    grantee_id: Uuid,
    grantee_email: String,
    grantee_display_name: String,
    created_at: DateTime<Utc>,
}

impl From<FileShareRow> for FileShare {
    fn from(row: FileShareRow) -> Self {
        Self {
            file_id: row.file_id,
            grantee: FileUser {
                id: row.grantee_id,
                email: row.grantee_email,
                display_name: row.grantee_display_name,
            },
            created_at: row.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct SharedFileRow {
    id: Uuid,
    filename: String,
    content_type: String,
    size_bytes: i64,
    checksum_sha256: Option<String>,
    object_key: String,
    state: String,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
    parent_folder_id: Option<Uuid>,
    owner_id: Uuid,
    owner_email: String,
    owner_display_name: String,
}

impl From<SharedFileRow> for SharedFile {
    fn from(row: SharedFileRow) -> Self {
        Self {
            file: DriveFile {
                id: row.id,
                filename: row.filename,
                parent_folder_id: row.parent_folder_id,
                content_type: row.content_type,
                size_bytes: row.size_bytes,
                checksum_sha256: row.checksum_sha256,
                object_key: row.object_key,
                state: row.state.into(),
                created_at: row.created_at,
                updated_at: row.updated_at,
                completed_at: row.completed_at,
                deleted_at: row.deleted_at,
            },
            owner: FileUser {
                id: row.owner_id,
                email: row.owner_email,
                display_name: row.owner_display_name,
            },
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct SearchFileRow {
    id: Uuid,
    filename: String,
    parent_folder_id: Option<Uuid>,
    content_type: String,
    size_bytes: i64,
    checksum_sha256: Option<String>,
    object_key: String,
    state: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
    access: String,
    owner_id: Option<Uuid>,
    owner_email: Option<String>,
    owner_display_name: Option<String>,
}

impl From<SearchFileRow> for SearchFileResult {
    fn from(row: SearchFileRow) -> Self {
        let access = if row.access == SearchAccess::Shared.as_str() {
            SearchAccess::Shared
        } else {
            SearchAccess::Owned
        };
        let owner = match (row.owner_id, row.owner_email, row.owner_display_name) {
            (Some(id), Some(email), Some(display_name)) => Some(FileUser {
                id,
                email,
                display_name,
            }),
            _ => None,
        };

        Self {
            file: DriveFile {
                id: row.id,
                filename: row.filename,
                parent_folder_id: row.parent_folder_id,
                content_type: row.content_type,
                size_bytes: row.size_bytes,
                checksum_sha256: row.checksum_sha256,
                object_key: row.object_key,
                state: row.state.into(),
                created_at: row.created_at,
                updated_at: row.updated_at,
                completed_at: row.completed_at,
                deleted_at: row.deleted_at,
            },
            access,
            owner,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ChangeLogRow {
    seq: i64,
    entity_type: String,
    entity_id: Uuid,
    op: String,
    occurred_at: DateTime<Utc>,
    file_id: Option<Uuid>,
    file_filename: Option<String>,
    file_parent_folder_id: Option<Uuid>,
    file_content_type: Option<String>,
    file_size_bytes: Option<i64>,
    file_checksum_sha256: Option<String>,
    file_object_key: Option<String>,
    file_state: Option<String>,
    file_created_at: Option<DateTime<Utc>>,
    file_updated_at: Option<DateTime<Utc>>,
    file_completed_at: Option<DateTime<Utc>>,
    file_deleted_at: Option<DateTime<Utc>>,
    folder_id: Option<Uuid>,
    folder_name: Option<String>,
    folder_parent_folder_id: Option<Uuid>,
    folder_created_at: Option<DateTime<Utc>>,
    folder_updated_at: Option<DateTime<Utc>>,
    folder_deleted_at: Option<DateTime<Utc>>,
}

impl From<ChangeLogRow> for ChangeLogEntry {
    fn from(row: ChangeLogRow) -> Self {
        let file = match (
            row.file_id,
            row.file_filename,
            row.file_content_type,
            row.file_size_bytes,
            row.file_object_key,
            row.file_state,
            row.file_created_at,
            row.file_updated_at,
        ) {
            (
                Some(id),
                Some(filename),
                Some(content_type),
                Some(size_bytes),
                Some(object_key),
                Some(state),
                Some(created_at),
                Some(updated_at),
            ) => Some(DriveFile {
                id,
                filename,
                parent_folder_id: row.file_parent_folder_id,
                content_type,
                size_bytes,
                checksum_sha256: row.file_checksum_sha256,
                object_key,
                state: state.into(),
                created_at,
                updated_at,
                completed_at: row.file_completed_at,
                deleted_at: row.file_deleted_at,
            }),
            _ => None,
        };

        let folder = match (
            row.folder_id,
            row.folder_name,
            row.folder_created_at,
            row.folder_updated_at,
        ) {
            (Some(id), Some(name), Some(created_at), Some(updated_at)) => Some(Folder {
                id,
                name,
                parent_folder_id: row.folder_parent_folder_id,
                created_at,
                updated_at,
                deleted_at: row.folder_deleted_at,
            }),
            _ => None,
        };

        Self {
            seq: row.seq,
            entity_type: ChangeEntityType::from(row.entity_type),
            entity_id: row.entity_id,
            op: ChangeOp::from(row.op),
            occurred_at: row.occurred_at,
            file,
            folder,
        }
    }
}

#[async_trait]
impl FileRepository for PostgresFileRepository {
    async fn pending_bytes_for_owner(
        &self,
        owner_id: Uuid,
        cap: i64,
    ) -> Result<i64, RepositoryError> {
        sqlx::query_scalar(
            "SELECT LEAST(COALESCE(SUM(size_bytes), 0), $2::numeric)::bigint
             FROM files
             WHERE owner_id = $1 AND state = 'pending'",
        )
        .bind(owner_id)
        .bind(cap)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)
    }

    async fn create_pending_file(
        &self,
        input: CreatePendingFileRecord,
    ) -> Result<PendingFile, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        Self::ensure_active_parent(&mut tx, input.owner_id, input.parent_folder_id).await?;

        let pending = sqlx::query_as::<_, PendingFileRow>(
            "INSERT INTO files (owner_id, filename, parent_folder_id, content_type, size_bytes, checksum_sha256, object_key, state)
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending')
             RETURNING id, size_bytes, object_key, state",
        )
        .bind(input.owner_id)
        .bind(&input.filename)
        .bind(input.parent_folder_id)
        .bind(&input.content_type)
        .bind(input.size_bytes)
        .bind(&input.checksum_sha256)
        .bind(&input.object_key)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(pending.into())
    }

    async fn create_resumable_upload(
        &self,
        input: CreateResumableUploadRecord,
    ) -> Result<ResumableUploadSession, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        Self::ensure_active_parent(&mut tx, input.owner_id, input.parent_folder_id).await?;

        let row = sqlx::query_as::<_, ResumableUploadRow>(
            "INSERT INTO files (
                owner_id, filename, parent_folder_id, content_type, size_bytes,
                checksum_sha256, object_key, state, upload_kind, multipart_upload_id,
                upload_expires_at, part_size_bytes
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending', 'resumable', $8, $9, $10)
             RETURNING id, owner_id, filename, parent_folder_id, content_type, size_bytes,
                checksum_sha256, object_key, state, created_at, updated_at, completed_at,
                multipart_upload_id, upload_expires_at, part_size_bytes",
        )
        .bind(input.owner_id)
        .bind(&input.filename)
        .bind(input.parent_folder_id)
        .bind(&input.content_type)
        .bind(input.size_bytes)
        .bind(&input.checksum_sha256)
        .bind(&input.object_key)
        .bind(&input.multipart_upload_id)
        .bind(input.upload_expires_at)
        .bind(input.part_size_bytes)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(row.into_session(Vec::new()))
    }

    async fn find_resumable_upload(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<Option<ResumableUploadSession>, RepositoryError> {
        let Some(row) = sqlx::query_as::<_, ResumableUploadRow>(
            "SELECT id, owner_id, filename, parent_folder_id, content_type, size_bytes,
                checksum_sha256, object_key, state, created_at, updated_at, completed_at,
                multipart_upload_id, upload_expires_at, part_size_bytes
             FROM files
             WHERE id = $1 AND owner_id = $2 AND upload_kind = 'resumable'",
        )
        .bind(file_id)
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?
        else {
            return Ok(None);
        };

        let parts = sqlx::query_as::<_, UploadPartRow>(
            "SELECT part_number, size_bytes, etag
             FROM upload_parts
             WHERE file_id = $1
             ORDER BY part_number ASC",
        )
        .bind(file_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?
        .into_iter()
        .map(Into::into)
        .collect();

        Ok(Some(row.into_session(parts)))
    }

    async fn record_upload_part(
        &self,
        input: RecordUploadPartRecord,
    ) -> Result<UploadPart, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;

        let upload: Option<(String, i64)> = sqlx::query_as(
            "SELECT state, size_bytes
             FROM files
             WHERE id = $1 AND owner_id = $2 AND upload_kind = 'resumable'
             FOR UPDATE",
        )
        .bind(input.file_id)
        .bind(input.owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        let Some((state, size_bytes)) = upload else {
            return Err(RepositoryError::NotFound);
        };
        if state != FileState::Pending.as_str()
            || input.size_bytes <= 0
            || input.size_bytes > size_bytes
        {
            return Err(RepositoryError::InvalidState);
        }

        let part = sqlx::query_as::<_, UploadPartRow>(
            "INSERT INTO upload_parts (file_id, part_number, size_bytes, etag)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (file_id, part_number)
             DO UPDATE SET size_bytes = EXCLUDED.size_bytes, etag = EXCLUDED.etag, updated_at = now()
             RETURNING part_number, size_bytes, etag",
        )
        .bind(input.file_id)
        .bind(input.part_number)
        .bind(input.size_bytes)
        .bind(input.etag)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(part.into())
    }

    async fn complete_resumable_upload_once(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
        expected_size: i64,
    ) -> Result<DriveFile, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        let locked = sqlx::query_as::<_, PendingFileRow>(
            "SELECT id, size_bytes, object_key, state
             FROM files
             WHERE id = $1 AND owner_id = $2 AND upload_kind = 'resumable'
             FOR UPDATE",
        )
        .bind(file_id)
        .bind(owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(RepositoryError::NotFound)?;

        if locked.state != FileState::Pending.as_str() || locked.size_bytes != expected_size {
            return Err(RepositoryError::InvalidState);
        }

        let updated = sqlx::query_as::<_, DriveFileRow>(
            "UPDATE files
             SET state = 'complete', completed_at = now(), updated_at = now()
             WHERE id = $1 AND owner_id = $2 AND state = 'pending'
             RETURNING id, filename, parent_folder_id, content_type, size_bytes, checksum_sha256, object_key, state, created_at, updated_at, completed_at, deleted_at",
        )
        .bind(file_id)
        .bind(owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(RepositoryError::InvalidState)?;

        sqlx::query(
            "UPDATE users
             SET storage_used_bytes = storage_used_bytes + $1
             WHERE id = $2",
        )
        .bind(locked.size_bytes)
        .bind(owner_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        Self::record_change(&mut tx, owner_id, "file", file_id, "upsert").await?;

        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(updated.into())
    }

    async fn expire_resumable_uploads(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<ExpiredUploadRecord>, RepositoryError> {
        let rows = sqlx::query_as::<_, (Uuid, String, String)>(
            "UPDATE files
             SET state = 'expired', updated_at = now()
             WHERE id IN (
                SELECT id
                FROM files
                WHERE upload_kind = 'resumable'
                  AND state = 'pending'
                  AND upload_expires_at < $1
                ORDER BY upload_expires_at ASC, id ASC
                LIMIT $2
                FOR UPDATE SKIP LOCKED
             )
             RETURNING id, object_key, multipart_upload_id",
        )
        .bind(now)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(
                |(file_id, object_key, multipart_upload_id)| ExpiredUploadRecord {
                    file_id,
                    object_key,
                    multipart_upload_id,
                },
            )
            .collect())
    }

    async fn create_folder(&self, input: CreateFolderRecord) -> Result<Folder, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        Self::ensure_active_parent(&mut tx, input.owner_id, input.parent_folder_id).await?;

        let folder = sqlx::query_as::<_, FolderRow>(
            "INSERT INTO folders (owner_id, name, parent_folder_id)
             VALUES ($1, $2, $3)
             RETURNING id, name, parent_folder_id, created_at, updated_at, deleted_at",
        )
        .bind(input.owner_id)
        .bind(&input.name)
        .bind(input.parent_folder_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        Self::record_change(&mut tx, input.owner_id, "folder", folder.id, "upsert").await?;

        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(folder.into())
    }

    async fn browse_folder(
        &self,
        owner_id: Uuid,
        parent_folder_id: Option<Uuid>,
    ) -> Result<DriveBrowse, RepositoryError> {
        if parent_folder_id.is_some() {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS (
                    SELECT 1 FROM folders
                    WHERE id = $1 AND owner_id = $2 AND deleted_at IS NULL
                )",
            )
            .bind(parent_folder_id)
            .bind(owner_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

            if !exists {
                return Err(RepositoryError::NotFound);
            }
        }

        let folders = sqlx::query_as::<_, FolderRow>(
            "SELECT id, name, parent_folder_id, created_at, updated_at, deleted_at
             FROM folders
             WHERE owner_id = $1
               AND parent_folder_id IS NOT DISTINCT FROM $2
               AND deleted_at IS NULL
             ORDER BY name ASC, created_at DESC, id DESC",
        )
        .bind(owner_id)
        .bind(parent_folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        let files = sqlx::query_as::<_, DriveFileRow>(
            "SELECT id, filename, parent_folder_id, content_type, size_bytes, checksum_sha256, object_key, state, created_at, updated_at, completed_at, deleted_at
             FROM files
             WHERE owner_id = $1
               AND parent_folder_id IS NOT DISTINCT FROM $2
               AND state = 'complete'
               AND deleted_at IS NULL
             ORDER BY completed_at DESC, id DESC",
        )
        .bind(owner_id)
        .bind(parent_folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(DriveBrowse {
            parent_folder_id,
            breadcrumbs: self.breadcrumbs(owner_id, parent_folder_id).await?,
            folders: folders.into_iter().map(Into::into).collect(),
            files: files.into_iter().map(Into::into).collect(),
        })
    }

    async fn list_active_folders(&self, owner_id: Uuid) -> Result<Vec<Folder>, RepositoryError> {
        sqlx::query_as::<_, FolderRow>(
            "SELECT id, name, parent_folder_id, created_at, updated_at, deleted_at
             FROM folders
             WHERE owner_id = $1 AND deleted_at IS NULL
             ORDER BY name ASC, created_at ASC, id ASC",
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(map_sqlx_error)
    }

    async fn complete_upload_once(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
        expected_size: i64,
    ) -> Result<DriveFile, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        let locked = sqlx::query_as::<_, PendingFileRow>(
            "SELECT id, size_bytes, object_key, state
             FROM files
             WHERE id = $1 AND owner_id = $2
             FOR UPDATE",
        )
        .bind(file_id)
        .bind(owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(RepositoryError::NotFound)?;

        if locked.state != FileState::Pending.as_str() || locked.size_bytes != expected_size {
            return Err(RepositoryError::InvalidState);
        }

        let updated = sqlx::query_as::<_, DriveFileRow>(
            "UPDATE files
             SET state = 'complete', completed_at = now(), updated_at = now()
             WHERE id = $1 AND owner_id = $2 AND state = 'pending'
             RETURNING id, filename, parent_folder_id, content_type, size_bytes, checksum_sha256, object_key, state, created_at, updated_at, completed_at, deleted_at",
        )
        .bind(file_id)
        .bind(owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(RepositoryError::InvalidState)?;

        sqlx::query(
            "UPDATE users
             SET storage_used_bytes = storage_used_bytes + $1
             WHERE id = $2",
        )
        .bind(locked.size_bytes)
        .bind(owner_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        Self::record_change(&mut tx, owner_id, "file", file_id, "upsert").await?;

        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(updated.into())
    }

    async fn find_owned_file_for_completion(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<Option<PendingFile>, RepositoryError> {
        sqlx::query_as::<_, PendingFileRow>(
            "SELECT id, size_bytes, object_key, state
             FROM files
             WHERE id = $1 AND owner_id = $2",
        )
        .bind(file_id)
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(Into::into))
        .map_err(map_sqlx_error)
    }

    async fn list_completed_files(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<DriveFile>, RepositoryError> {
        sqlx::query_as::<_, DriveFileRow>(
            "SELECT id, filename, parent_folder_id, content_type, size_bytes, checksum_sha256, object_key, state, created_at, updated_at, completed_at, deleted_at
             FROM files
             WHERE owner_id = $1 AND state = 'complete' AND deleted_at IS NULL
             ORDER BY completed_at DESC, id DESC",
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(map_sqlx_error)
    }

    async fn find_completed_owned_file(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<Option<DriveFile>, RepositoryError> {
        sqlx::query_as::<_, DriveFileRow>(
            "SELECT id, filename, parent_folder_id, content_type, size_bytes, checksum_sha256, object_key, state, created_at, updated_at, completed_at, deleted_at
             FROM files
             WHERE id = $1 AND owner_id = $2 AND state = 'complete' AND deleted_at IS NULL",
        )
        .bind(file_id)
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(Into::into))
        .map_err(map_sqlx_error)
    }

    async fn update_owned_file(
        &self,
        input: UpdateFileRecord,
    ) -> Result<DriveFile, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        Self::ensure_active_parent(&mut tx, input.owner_id, input.parent_folder_id.flatten())
            .await?;

        let file = sqlx::query_as::<_, DriveFileRow>(
            "UPDATE files
             SET filename = COALESCE($3, filename),
                 parent_folder_id = CASE WHEN $4 THEN $5 ELSE parent_folder_id END,
                 updated_at = now()
             WHERE id = $1
               AND owner_id = $2
               AND state = 'complete'
               AND deleted_at IS NULL
             RETURNING id, filename, parent_folder_id, content_type, size_bytes, checksum_sha256, object_key, state, created_at, updated_at, completed_at, deleted_at",
        )
        .bind(input.file_id)
        .bind(input.owner_id)
        .bind(input.filename.as_deref())
        .bind(input.parent_folder_id.is_some())
        .bind(input.parent_folder_id.flatten())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(RepositoryError::NotFound)?;

        Self::record_change(&mut tx, input.owner_id, "file", file.id, "upsert").await?;

        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(file.into())
    }

    async fn update_owned_folder(
        &self,
        input: UpdateFolderRecord,
    ) -> Result<Folder, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;

        let current: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM folders
             WHERE id = $1 AND owner_id = $2 AND deleted_at IS NULL
             FOR UPDATE",
        )
        .bind(input.folder_id)
        .bind(input.owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        if current.is_none() {
            return Err(RepositoryError::NotFound);
        }

        if let Some(parent_folder_id) = input.parent_folder_id {
            Self::ensure_active_parent(&mut tx, input.owner_id, parent_folder_id).await?;
            if parent_folder_id == Some(input.folder_id) {
                return Err(RepositoryError::InvalidState);
            }
            if let Some(parent_folder_id) = parent_folder_id {
                let is_descendant: bool = sqlx::query_scalar(
                    "WITH RECURSIVE descendants AS (
                        SELECT id FROM folders
                        WHERE parent_folder_id = $1
                          AND owner_id = $2
                          AND deleted_at IS NULL
                        UNION ALL
                        SELECT child.id FROM folders child
                        JOIN descendants d ON child.parent_folder_id = d.id
                        WHERE child.owner_id = $2
                          AND child.deleted_at IS NULL
                     )
                     SELECT EXISTS (SELECT 1 FROM descendants WHERE id = $3)",
                )
                .bind(input.folder_id)
                .bind(input.owner_id)
                .bind(parent_folder_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;

                if is_descendant {
                    return Err(RepositoryError::InvalidState);
                }
            }
        }

        let folder = sqlx::query_as::<_, FolderRow>(
            "UPDATE folders
             SET name = COALESCE($3, name),
                 parent_folder_id = CASE WHEN $4 THEN $5 ELSE parent_folder_id END,
                 updated_at = now()
             WHERE id = $1
               AND owner_id = $2
               AND deleted_at IS NULL
             RETURNING id, name, parent_folder_id, created_at, updated_at, deleted_at",
        )
        .bind(input.folder_id)
        .bind(input.owner_id)
        .bind(input.name.as_deref())
        .bind(input.parent_folder_id.is_some())
        .bind(input.parent_folder_id.flatten())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(RepositoryError::NotFound)?;

        Self::record_change(&mut tx, input.owner_id, "folder", folder.id, "upsert").await?;

        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(folder.into())
    }

    async fn find_downloadable_file(
        &self,
        user_id: Uuid,
        file_id: Uuid,
    ) -> Result<Option<DriveFile>, RepositoryError> {
        sqlx::query_as::<_, DriveFileRow>(
            "SELECT id, filename, parent_folder_id, content_type, size_bytes, checksum_sha256, object_key, state, created_at, updated_at, completed_at, deleted_at
             FROM files f
             WHERE f.id = $1
               AND f.state = 'complete'
               AND f.deleted_at IS NULL
               AND (
                   f.owner_id = $2
                   OR EXISTS (
                       SELECT 1 FROM file_shares fs
                       WHERE fs.file_id = f.id AND fs.grantee_id = $2
                   )
               )",
        )
        .bind(file_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(Into::into))
        .map_err(map_sqlx_error)
    }

    async fn soft_delete_owned_file(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        let result = sqlx::query(
            "UPDATE files
             SET deleted_at = now(), deleted_by_folder_id = NULL, updated_at = now()
             WHERE id = $1 AND owner_id = $2 AND state = 'complete' AND deleted_at IS NULL",
        )
        .bind(file_id)
        .bind(owner_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        Self::record_change(&mut tx, owner_id, "file", file_id, "delete").await?;

        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn restore_owned_file(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<DriveFile, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        let file: DriveFile = sqlx::query_as::<_, DriveFileRow>(
            "UPDATE files
             SET deleted_at = NULL, updated_at = now()
             WHERE id = $1
               AND owner_id = $2
               AND deleted_at IS NOT NULL
               AND deleted_by_folder_id IS NULL
               AND (
                   parent_folder_id IS NULL
                   OR EXISTS (
                       SELECT 1 FROM folders parent
                       WHERE parent.id = files.parent_folder_id
                         AND parent.owner_id = files.owner_id
                         AND parent.deleted_at IS NULL
                   )
               )
             RETURNING id, filename, parent_folder_id, content_type, size_bytes, checksum_sha256, object_key, state, created_at, updated_at, completed_at, deleted_at",
        )
        .bind(file_id)
        .bind(owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .map(Into::into)
        .ok_or(RepositoryError::NotFound)?;

        Self::record_change(&mut tx, owner_id, "file", file_id, "upsert").await?;

        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(file)
    }

    async fn list_trash(&self, owner_id: Uuid) -> Result<Vec<DriveFile>, RepositoryError> {
        sqlx::query_as::<_, DriveFileRow>(
            "SELECT id, filename, parent_folder_id, content_type, size_bytes, checksum_sha256, object_key, state, created_at, updated_at, completed_at, deleted_at
             FROM files
             WHERE owner_id = $1 AND deleted_at IS NOT NULL
             ORDER BY deleted_at DESC, id DESC",
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(map_sqlx_error)
    }

    async fn soft_delete_owned_folder_tree(
        &self,
        owner_id: Uuid,
        folder_id: Uuid,
    ) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        let root_exists: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM folders
             WHERE id = $1 AND owner_id = $2 AND deleted_at IS NULL
             FOR UPDATE",
        )
        .bind(folder_id)
        .bind(owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        if root_exists.is_none() {
            return Err(RepositoryError::NotFound);
        }

        let deleted_folders: Vec<(Uuid,)> = sqlx::query_as(
            "WITH RECURSIVE descendants AS (
                SELECT id FROM folders
                WHERE id = $1 AND owner_id = $2 AND deleted_at IS NULL
                UNION ALL
                SELECT child.id FROM folders child
                JOIN descendants d ON child.parent_folder_id = d.id
                WHERE child.owner_id = $2
                  AND child.deleted_at IS NULL
             )
             UPDATE folders
             SET deleted_at = now(),
                 deleted_by_folder_id = $1,
                 updated_at = now()
             WHERE owner_id = $2
               AND deleted_at IS NULL
               AND id IN (SELECT id FROM descendants)
             RETURNING id",
        )
        .bind(folder_id)
        .bind(owner_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        let deleted_files: Vec<(Uuid,)> = sqlx::query_as(
            "WITH RECURSIVE descendants AS (
                SELECT id FROM folders
                WHERE id = $1 AND owner_id = $2
                UNION ALL
                SELECT child.id FROM folders child
                JOIN descendants d ON child.parent_folder_id = d.id
                WHERE child.owner_id = $2
             )
             UPDATE files
             SET deleted_at = now(),
                 deleted_by_folder_id = $1,
                 updated_at = now()
             WHERE owner_id = $2
               AND state = 'complete'
               AND deleted_at IS NULL
               AND parent_folder_id IN (SELECT id FROM descendants)
             RETURNING id",
        )
        .bind(folder_id)
        .bind(owner_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        for (folder_node_id,) in deleted_folders {
            Self::record_change(&mut tx, owner_id, "folder", folder_node_id, "delete").await?;
        }
        for (file_node_id,) in deleted_files {
            Self::record_change(&mut tx, owner_id, "file", file_node_id, "delete").await?;
        }

        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn restore_owned_folder_tree(
        &self,
        owner_id: Uuid,
        folder_id: Uuid,
    ) -> Result<Folder, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        let parent_deleted: Option<bool> = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM folders parent
                WHERE parent.id = folders.parent_folder_id
                  AND parent.owner_id = folders.owner_id
                  AND parent.deleted_at IS NOT NULL
             )
             FROM folders
             WHERE id = $1
               AND owner_id = $2
               AND deleted_at IS NOT NULL
               AND deleted_by_folder_id = $1
             FOR UPDATE",
        )
        .bind(folder_id)
        .bind(owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        let Some(parent_deleted) = parent_deleted else {
            return Err(RepositoryError::NotFound);
        };
        if parent_deleted {
            return Err(RepositoryError::InvalidState);
        }

        let restored_files: Vec<(Uuid,)> = sqlx::query_as(
            "UPDATE files
             SET deleted_at = NULL,
                 deleted_by_folder_id = NULL,
                 updated_at = now()
             WHERE owner_id = $1
               AND deleted_by_folder_id = $2
             RETURNING id",
        )
        .bind(owner_id)
        .bind(folder_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        let restored_folders = sqlx::query_as::<_, FolderRow>(
            "UPDATE folders
             SET deleted_at = NULL,
                 deleted_by_folder_id = NULL,
                 updated_at = now()
             WHERE owner_id = $1
               AND deleted_by_folder_id = $2
             RETURNING id, name, parent_folder_id, created_at, updated_at, deleted_at",
        )
        .bind(owner_id)
        .bind(folder_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        let folder = restored_folders
            .iter()
            .find(|folder| folder.id == folder_id)
            .cloned()
            .ok_or(RepositoryError::NotFound)?;

        for folder_node in &restored_folders {
            Self::record_change(&mut tx, owner_id, "folder", folder_node.id, "upsert").await?;
        }
        for (file_node_id,) in restored_files {
            Self::record_change(&mut tx, owner_id, "file", file_node_id, "upsert").await?;
        }

        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(folder.into())
    }

    async fn list_drive_trash(&self, owner_id: Uuid) -> Result<DriveBrowse, RepositoryError> {
        let folders = sqlx::query_as::<_, FolderRow>(
            "SELECT id, name, parent_folder_id, created_at, updated_at, deleted_at
             FROM folders
             WHERE owner_id = $1
               AND deleted_at IS NOT NULL
               AND deleted_by_folder_id = id
             ORDER BY deleted_at DESC, id DESC",
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        let files = sqlx::query_as::<_, DriveFileRow>(
            "SELECT id, filename, parent_folder_id, content_type, size_bytes, checksum_sha256, object_key, state, created_at, updated_at, completed_at, deleted_at
             FROM files
             WHERE owner_id = $1
               AND deleted_at IS NOT NULL
               AND deleted_by_folder_id IS NULL
             ORDER BY deleted_at DESC, id DESC",
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(DriveBrowse {
            parent_folder_id: None,
            breadcrumbs: Vec::new(),
            folders: folders.into_iter().map(Into::into).collect(),
            files: files.into_iter().map(Into::into).collect(),
        })
    }

    async fn create_share(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
        grantee_id: Uuid,
    ) -> Result<FileShare, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                 SELECT 1 FROM files
                 WHERE id = $1 AND owner_id = $2 AND state = 'complete' AND deleted_at IS NULL
             )",
        )
        .bind(file_id)
        .bind(owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        if !exists {
            return Err(RepositoryError::NotFound);
        }

        sqlx::query(
            "INSERT INTO file_shares (file_id, grantee_id)
             VALUES ($1, $2)
             ON CONFLICT (file_id, grantee_id) DO NOTHING",
        )
        .bind(file_id)
        .bind(grantee_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        let share = sqlx::query_as::<_, FileShareRow>(
            "SELECT fs.file_id, u.id AS grantee_id, u.email AS grantee_email, u.display_name AS grantee_display_name, fs.created_at
             FROM file_shares fs
             JOIN users u ON u.id = fs.grantee_id
             WHERE fs.file_id = $1 AND fs.grantee_id = $2",
        )
        .bind(file_id)
        .bind(grantee_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(share.into())
    }

    async fn list_shares(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<Vec<FileShare>, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        let owned: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM files WHERE id = $1 AND owner_id = $2)",
        )
        .bind(file_id)
        .bind(owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        if !owned {
            return Err(RepositoryError::NotFound);
        }

        let shares = sqlx::query_as::<_, FileShareRow>(
            "SELECT fs.file_id, u.id AS grantee_id, u.email AS grantee_email, u.display_name AS grantee_display_name, fs.created_at
             FROM file_shares fs
             JOIN users u ON u.id = fs.grantee_id
             WHERE fs.file_id = $1
             ORDER BY fs.created_at DESC, u.id DESC",
        )
        .bind(file_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(shares.into_iter().map(Into::into).collect())
    }

    async fn revoke_share(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
        grantee_id: Uuid,
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            "DELETE FROM file_shares fs
             USING files f
             WHERE fs.file_id = $1
               AND fs.grantee_id = $2
               AND f.id = fs.file_id
               AND f.owner_id = $3",
        )
        .bind(file_id)
        .bind(grantee_id)
        .bind(owner_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            Err(RepositoryError::NotFound)
        } else {
            Ok(())
        }
    }

    async fn list_shared_with_me(
        &self,
        grantee_id: Uuid,
    ) -> Result<Vec<SharedFile>, RepositoryError> {
        sqlx::query_as::<_, SharedFileRow>(
            "SELECT f.id, f.filename, f.parent_folder_id, f.content_type, f.size_bytes, f.checksum_sha256, f.object_key,
                    f.state, f.created_at, f.updated_at, f.completed_at, f.deleted_at,
                    owner.id AS owner_id, owner.email AS owner_email, owner.display_name AS owner_display_name
             FROM file_shares fs
             JOIN files f ON f.id = fs.file_id
             JOIN users owner ON owner.id = f.owner_id
             WHERE fs.grantee_id = $1 AND f.state = 'complete' AND f.deleted_at IS NULL
             ORDER BY fs.created_at DESC, f.id DESC",
        )
        .bind(grantee_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(map_sqlx_error)
    }

    async fn search_accessible_files(
        &self,
        input: SearchFilesRecord,
    ) -> Result<Vec<SearchFileResult>, RepositoryError> {
        let escaped_query = Self::escape_like_literal(&input.query.to_lowercase());
        sqlx::query_as::<_, SearchFileRow>(
            "WITH accessible AS (
                SELECT f.id, f.filename, f.parent_folder_id, f.content_type, f.size_bytes, f.checksum_sha256,
                       f.object_key, f.state, f.created_at, f.updated_at, f.completed_at, f.deleted_at,
                       'owned'::text AS access,
                       NULL::uuid AS owner_id,
                       NULL::text AS owner_email,
                       NULL::text AS owner_display_name
                FROM files f
                WHERE f.owner_id = $1
                  AND f.state = 'complete'
                  AND ($3 OR f.deleted_at IS NULL)
                  AND lower(f.filename) LIKE '%' || $2 || '%' ESCAPE '\\'
                UNION ALL
                SELECT f.id, f.filename, f.parent_folder_id, f.content_type, f.size_bytes, f.checksum_sha256,
                       f.object_key, f.state, f.created_at, f.updated_at, f.completed_at, f.deleted_at,
                       'shared'::text AS access,
                       owner.id AS owner_id,
                       owner.email AS owner_email,
                       owner.display_name AS owner_display_name
                FROM file_shares fs
                JOIN files f ON f.id = fs.file_id
                JOIN users owner ON owner.id = f.owner_id
                WHERE fs.grantee_id = $1
                  AND f.state = 'complete'
                  AND f.deleted_at IS NULL
                  AND lower(f.filename) LIKE '%' || $2 || '%' ESCAPE '\\'
             )
             SELECT id, filename, parent_folder_id, content_type, size_bytes, checksum_sha256,
                    object_key, state, created_at, updated_at, completed_at, deleted_at,
                    access, owner_id, owner_email, owner_display_name
             FROM accessible
             ORDER BY
               CASE
                 WHEN lower(filename) = $2 THEN 0
                 WHEN lower(filename) LIKE $2 || '%' ESCAPE '\\' THEN 1
                 ELSE 2
               END,
               completed_at DESC NULLS LAST,
               id DESC
             LIMIT $4",
        )
        .bind(input.user_id)
        .bind(escaped_query)
        .bind(input.include_deleted)
        .bind(input.limit)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(map_sqlx_error)
    }

    async fn list_changes(
        &self,
        owner_id: Uuid,
        after_seq: i64,
        limit: i64,
    ) -> Result<Vec<ChangeLogEntry>, RepositoryError> {
        sqlx::query_as::<_, ChangeLogRow>(
            "SELECT cl.seq, cl.entity_type, cl.entity_id, cl.op, cl.occurred_at,
                    f.id AS file_id, f.filename AS file_filename,
                    f.parent_folder_id AS file_parent_folder_id,
                    f.content_type AS file_content_type, f.size_bytes AS file_size_bytes,
                    f.checksum_sha256 AS file_checksum_sha256, f.object_key AS file_object_key,
                    f.state AS file_state, f.created_at AS file_created_at,
                    f.updated_at AS file_updated_at, f.completed_at AS file_completed_at,
                    f.deleted_at AS file_deleted_at,
                    fo.id AS folder_id, fo.name AS folder_name,
                    fo.parent_folder_id AS folder_parent_folder_id,
                    fo.created_at AS folder_created_at, fo.updated_at AS folder_updated_at,
                    fo.deleted_at AS folder_deleted_at
             FROM change_log cl
             LEFT JOIN files f
                    ON cl.entity_type = 'file' AND cl.op = 'upsert' AND f.id = cl.entity_id
             LEFT JOIN folders fo
                    ON cl.entity_type = 'folder' AND cl.op = 'upsert' AND fo.id = cl.entity_id
             WHERE cl.owner_id = $1 AND cl.seq > $2
             ORDER BY cl.seq ASC
             LIMIT $3",
        )
        .bind(owner_id)
        .bind(after_seq)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(map_sqlx_error)
    }
}
