use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::adapters::postgres::tx::map_sqlx_error;
use crate::application::ports::RepositoryError;
use crate::application::ports::files::{CreatePendingFileRecord, FileRepository};
use crate::domain::files::{DriveFile, FileState, PendingFile};

#[derive(Debug, Clone)]
pub struct PostgresFileRepository {
    pool: PgPool,
}

impl PostgresFileRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
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
    content_type: String,
    size_bytes: i64,
    checksum_sha256: Option<String>,
    object_key: String,
    state: String,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

impl From<DriveFileRow> for DriveFile {
    fn from(row: DriveFileRow) -> Self {
        Self {
            id: row.id,
            filename: row.filename,
            content_type: row.content_type,
            size_bytes: row.size_bytes,
            checksum_sha256: row.checksum_sha256,
            object_key: row.object_key,
            state: row.state.into(),
            created_at: row.created_at,
            completed_at: row.completed_at,
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
        sqlx::query_as::<_, PendingFileRow>(
            "INSERT INTO files (owner_id, filename, content_type, size_bytes, checksum_sha256, object_key, state)
             VALUES ($1, $2, $3, $4, $5, $6, 'pending')
             RETURNING id, size_bytes, object_key, state",
        )
        .bind(input.owner_id)
        .bind(&input.filename)
        .bind(&input.content_type)
        .bind(input.size_bytes)
        .bind(&input.checksum_sha256)
        .bind(&input.object_key)
        .fetch_one(&self.pool)
        .await
        .map(Into::into)
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
             SET state = 'complete', completed_at = now()
             WHERE id = $1 AND owner_id = $2 AND state = 'pending'
             RETURNING id, filename, content_type, size_bytes, checksum_sha256, object_key, state, created_at, completed_at",
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
            "SELECT id, filename, content_type, size_bytes, checksum_sha256, object_key, state, created_at, completed_at
             FROM files
             WHERE owner_id = $1 AND state = 'complete'
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
            "SELECT id, filename, content_type, size_bytes, checksum_sha256, object_key, state, created_at, completed_at
             FROM files
             WHERE id = $1 AND owner_id = $2 AND state = 'complete'",
        )
        .bind(file_id)
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(Into::into))
        .map_err(map_sqlx_error)
    }
}
