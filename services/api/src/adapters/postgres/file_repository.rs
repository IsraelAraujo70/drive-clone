use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::adapters::postgres::tx::map_sqlx_error;
use crate::application::ports::RepositoryError;
use crate::application::ports::files::{CreatePendingFileRecord, FileRepository};
use crate::domain::files::{DriveFile, FileShare, FileState, FileUser, PendingFile, SharedFile};

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
    deleted_at: Option<DateTime<Utc>>,
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
            deleted_at: row.deleted_at,
        }
    }
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
                content_type: row.content_type,
                size_bytes: row.size_bytes,
                checksum_sha256: row.checksum_sha256,
                object_key: row.object_key,
                state: row.state.into(),
                created_at: row.created_at,
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
             RETURNING id, filename, content_type, size_bytes, checksum_sha256, object_key, state, created_at, completed_at, deleted_at",
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
            "SELECT id, filename, content_type, size_bytes, checksum_sha256, object_key, state, created_at, completed_at, deleted_at
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
            "SELECT id, filename, content_type, size_bytes, checksum_sha256, object_key, state, created_at, completed_at, deleted_at
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

    async fn find_downloadable_file(
        &self,
        user_id: Uuid,
        file_id: Uuid,
    ) -> Result<Option<DriveFile>, RepositoryError> {
        sqlx::query_as::<_, DriveFileRow>(
            "SELECT id, filename, content_type, size_bytes, checksum_sha256, object_key, state, created_at, completed_at, deleted_at
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
        let result = sqlx::query(
            "UPDATE files
             SET deleted_at = now()
             WHERE id = $1 AND owner_id = $2 AND state = 'complete' AND deleted_at IS NULL",
        )
        .bind(file_id)
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

    async fn restore_owned_file(
        &self,
        owner_id: Uuid,
        file_id: Uuid,
    ) -> Result<DriveFile, RepositoryError> {
        sqlx::query_as::<_, DriveFileRow>(
            "UPDATE files
             SET deleted_at = NULL
             WHERE id = $1 AND owner_id = $2 AND deleted_at IS NOT NULL
             RETURNING id, filename, content_type, size_bytes, checksum_sha256, object_key, state, created_at, completed_at, deleted_at",
        )
        .bind(file_id)
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(Into::into))
        .map_err(map_sqlx_error)?
        .ok_or(RepositoryError::NotFound)
    }

    async fn list_trash(&self, owner_id: Uuid) -> Result<Vec<DriveFile>, RepositoryError> {
        sqlx::query_as::<_, DriveFileRow>(
            "SELECT id, filename, content_type, size_bytes, checksum_sha256, object_key, state, created_at, completed_at, deleted_at
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
            "SELECT f.id, f.filename, f.content_type, f.size_bytes, f.checksum_sha256, f.object_key,
                    f.state, f.created_at, f.completed_at, f.deleted_at,
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
}
