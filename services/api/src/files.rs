use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::AppState;
use crate::auth::{ApiError, Auth};

const FILENAME_MAX_CHARS: usize = 255;
const CONTENT_TYPE_MAX_CHARS: usize = 255;

#[derive(Debug, Deserialize)]
pub struct CreateUploadRequest {
    filename: String,
    content_type: String,
    size_bytes: i64,
    checksum_sha256: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateUploadResponse {
    file_id: Uuid,
    upload_url: String,
    object_key: String,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct FileResponse {
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

#[derive(Debug, Serialize)]
pub struct ListFilesResponse {
    files: Vec<FileResponse>,
}

#[derive(Debug, Serialize)]
pub struct DownloadResponse {
    download_url: String,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct PendingFile {
    id: Uuid,
    size_bytes: i64,
    object_key: String,
    state: String,
}

pub async fn create_upload(
    State(state): State<AppState>,
    auth: Auth,
    Json(request): Json<CreateUploadRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let filename = validate_filename(&request.filename)?;
    let content_type = validate_content_type(&request.content_type)?;
    validate_size(request.size_bytes, state.max_file_size_bytes)?;
    let checksum_sha256 = validate_checksum(request.checksum_sha256.as_deref())?;

    ensure_quota(
        &state.pool,
        auth.user.id,
        auth.user.storage_used_bytes,
        auth.user.storage_quota_bytes,
        request.size_bytes,
    )
    .await?;

    let object_key = format!("{}/{}", auth.user.id, Uuid::new_v4());
    let presigned = state
        .storage
        .presign_put(
            &object_key,
            &content_type,
            request.size_bytes,
            state.presigned_url_ttl_seconds,
        )
        .await?;

    let row = sqlx::query_as::<_, PendingFile>(
        "INSERT INTO files (owner_id, filename, content_type, size_bytes, checksum_sha256, object_key, state)
         VALUES ($1, $2, $3, $4, $5, $6, 'pending')
         RETURNING id, size_bytes, object_key, state",
    )
    .bind(auth.user.id)
    .bind(&filename)
    .bind(&content_type)
    .bind(request.size_bytes)
    .bind(&checksum_sha256)
    .bind(&object_key)
    .fetch_one(&state.pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(CreateUploadResponse {
            file_id: row.id,
            upload_url: presigned.url,
            object_key: row.object_key,
            expires_at: presigned.expires_at,
        }),
    ))
}

pub async fn complete_upload(
    State(state): State<AppState>,
    auth: Auth,
    Path(file_id): Path<Uuid>,
) -> Result<Json<FileResponse>, ApiError> {
    let mut tx = state.pool.begin().await?;
    let row = fetch_owned_file_for_update(&mut tx, auth.user.id, file_id)
        .await?
        .ok_or(ApiError::FileNotFound)?;

    if row.state != "pending" {
        return Err(ApiError::InvalidFileState);
    }

    let object = state.storage.head_object(&row.object_key).await?;
    if object.content_length != row.size_bytes {
        return Err(ApiError::StorageError);
    }

    let updated = sqlx::query_as::<_, FileResponse>(
        "UPDATE files
         SET state = 'complete', completed_at = now()
         WHERE id = $1 AND owner_id = $2 AND state = 'pending'
         RETURNING id, filename, content_type, size_bytes, checksum_sha256, object_key, state, created_at, completed_at",
    )
    .bind(file_id)
    .bind(auth.user.id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::InvalidFileState)?;

    sqlx::query(
        "UPDATE users
         SET storage_used_bytes = storage_used_bytes + $1
         WHERE id = $2",
    )
    .bind(row.size_bytes)
    .bind(auth.user.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Json(updated))
}

pub async fn list_files(
    State(state): State<AppState>,
    auth: Auth,
) -> Result<Json<ListFilesResponse>, ApiError> {
    let files = sqlx::query_as::<_, FileResponse>(
        "SELECT id, filename, content_type, size_bytes, checksum_sha256, object_key, state, created_at, completed_at
         FROM files
         WHERE owner_id = $1 AND state = 'complete'
         ORDER BY completed_at DESC, id DESC",
    )
    .bind(auth.user.id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(ListFilesResponse { files }))
}

pub async fn download_file(
    State(state): State<AppState>,
    auth: Auth,
    Path(file_id): Path<Uuid>,
) -> Result<Json<DownloadResponse>, ApiError> {
    let file = sqlx::query_as::<_, FileResponse>(
        "SELECT id, filename, content_type, size_bytes, checksum_sha256, object_key, state, created_at, completed_at
         FROM files
         WHERE id = $1 AND owner_id = $2 AND state = 'complete'",
    )
    .bind(file_id)
    .bind(auth.user.id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::FileNotFound)?;

    let presigned = state
        .storage
        .presign_get(&file.object_key, state.presigned_url_ttl_seconds)
        .await?;

    Ok(Json(DownloadResponse {
        download_url: presigned.url,
        expires_at: presigned.expires_at,
    }))
}

async fn fetch_owned_file_for_update(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    file_id: Uuid,
) -> Result<Option<PendingFile>, ApiError> {
    sqlx::query_as::<_, PendingFile>(
        "SELECT id, size_bytes, object_key, state
         FROM files
         WHERE id = $1 AND owner_id = $2
         FOR UPDATE",
    )
    .bind(file_id)
    .bind(owner_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(ApiError::from)
}

async fn ensure_quota(
    pool: &PgPool,
    user_id: Uuid,
    storage_used_bytes: i64,
    storage_quota_bytes: i64,
    new_file_size: i64,
) -> Result<(), ApiError> {
    let pending_cap = storage_quota_bytes.max(0);
    let pending_bytes: i64 = sqlx::query_scalar(
        "SELECT LEAST(COALESCE(SUM(size_bytes), 0), $2::numeric)::bigint
         FROM files
         WHERE owner_id = $1 AND state = 'pending'",
    )
    .bind(user_id)
    .bind(pending_cap)
    .fetch_one(pool)
    .await?;

    if storage_used_bytes
        .checked_add(pending_bytes)
        .and_then(|used| used.checked_add(new_file_size))
        .is_some_and(|projected| projected <= storage_quota_bytes)
    {
        Ok(())
    } else {
        Err(ApiError::QuotaExceeded)
    }
}

fn validate_filename(filename: &str) -> Result<String, ApiError> {
    let filename = filename.trim();
    let valid = !filename.is_empty()
        && filename.chars().count() <= FILENAME_MAX_CHARS
        && !filename.contains('/')
        && !filename.contains('\\')
        && filename != "."
        && filename != "..";
    if valid {
        Ok(filename.to_string())
    } else {
        Err(ApiError::Validation("Enter a valid filename"))
    }
}

fn validate_content_type(content_type: &str) -> Result<String, ApiError> {
    let content_type = content_type.trim().to_ascii_lowercase();
    let valid = !content_type.is_empty()
        && content_type.len() <= CONTENT_TYPE_MAX_CHARS
        && content_type.contains('/')
        && !content_type.contains(char::is_whitespace);
    if valid {
        Ok(content_type)
    } else {
        Err(ApiError::Validation("Enter a valid content type"))
    }
}

fn validate_size(size_bytes: i64, max_file_size_bytes: i64) -> Result<(), ApiError> {
    if size_bytes <= 0 {
        Err(ApiError::Validation("File size must be greater than zero"))
    } else if size_bytes > max_file_size_bytes {
        Err(ApiError::FileTooLarge)
    } else {
        Ok(())
    }
}

fn validate_checksum(checksum: Option<&str>) -> Result<Option<String>, ApiError> {
    match checksum {
        None => Ok(None),
        Some(checksum)
            if checksum.len() == 64
                && checksum
                    .chars()
                    .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()) =>
        {
            Ok(Some(checksum.to_string()))
        }
        Some(_) => Err(ApiError::Validation("Enter a valid SHA-256 checksum")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use axum::Router;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use crate::storage::FakeStorage;

    #[test]
    fn filename_validation_rejects_empty_paths_and_parent_refs() {
        assert!(validate_filename("report.pdf").is_ok());
        for filename in ["", "   ", "../x", "a/b", "a\\b", ".", ".."] {
            assert!(
                validate_filename(filename).is_err(),
                "should reject {filename:?}"
            );
        }
    }

    #[test]
    fn checksum_validation_requires_lowercase_sha256_hex() {
        assert!(validate_checksum(Some(&"a".repeat(64))).is_ok());
        assert!(validate_checksum(None).unwrap().is_none());
        assert!(validate_checksum(Some(&"A".repeat(64))).is_err());
        assert!(validate_checksum(Some("abc")).is_err());
    }

    #[test]
    fn size_validation_enforces_positive_and_maximum() {
        assert!(validate_size(1, 10).is_ok());
        assert!(matches!(validate_size(0, 10), Err(ApiError::Validation(_))));
        assert!(matches!(validate_size(11, 10), Err(ApiError::FileTooLarge)));
    }

    async fn request(
        app: Router,
        method: &str,
        uri: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(token) = token {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        let request = match body {
            Some(body) => builder
                .header("content-type", "application/json")
                .body(Body::from(body.to_string())),
            None => builder.body(Body::empty()),
        }
        .unwrap();

        let response = app.oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, value)
    }

    async fn signup(app: Router, email: &str) -> String {
        let (status, body) = request(
            app,
            "POST",
            "/auth/signup",
            None,
            Some(json!({"email": email, "password": "password123", "display_name": "Test User"})),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        body["token"].as_str().unwrap().to_string()
    }

    fn upload_body(size_bytes: i64) -> Value {
        json!({
            "filename": "report.txt",
            "content_type": "text/plain",
            "size_bytes": size_bytes,
            "checksum_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        })
    }

    #[sqlx::test]
    async fn file_routes_require_auth(pool: PgPool) {
        let storage = Arc::new(FakeStorage::default());
        let app = crate::app_with_storage(pool, storage);

        for (method, uri, body) in [
            ("POST", "/files/uploads", Some(upload_body(10))),
            ("GET", "/files", None),
        ] {
            let (status, response) = request(app.clone(), method, uri, None, body).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED);
            assert_eq!(response["error"], "unauthorized");
        }
    }

    #[sqlx::test]
    async fn create_upload_validates_size_and_quota(pool: PgPool) {
        let storage = Arc::new(FakeStorage::default());
        let app = crate::app_with_storage(pool.clone(), storage);
        let token = signup(app.clone(), "quota@example.com").await;

        let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
            .bind("quota@example.com")
            .fetch_one(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE users SET storage_quota_bytes = 10 WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();

        let (status, body) = request(
            app.clone(),
            "POST",
            "/files/uploads",
            Some(&token),
            Some(upload_body(11)),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["error"], "quota_exceeded");

        let (status, pending_upload) = request(
            app.clone(),
            "POST",
            "/files/uploads",
            Some(&token),
            Some(upload_body(6)),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert!(pending_upload["file_id"].as_str().is_some());

        let (status, body) = request(
            app.clone(),
            "POST",
            "/files/uploads",
            Some(&token),
            Some(upload_body(5)),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["error"], "quota_exceeded");

        let app = crate::app_with_state(crate::AppState {
            pool,
            storage: Arc::new(FakeStorage::default()),
            max_file_size_bytes: 5,
            presigned_url_ttl_seconds: 900,
        });
        let (status, body) = request(
            app,
            "POST",
            "/files/uploads",
            Some(&token),
            Some(upload_body(6)),
        )
        .await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(body["error"], "file_too_large");
    }

    #[sqlx::test]
    async fn complete_upload_tracks_storage_once_and_lists_completed_files(pool: PgPool) {
        let storage = Arc::new(FakeStorage::default());
        let app = crate::app_with_storage(pool.clone(), storage.clone());
        let token = signup(app.clone(), "owner@example.com").await;

        let (status, upload) = request(
            app.clone(),
            "POST",
            "/files/uploads",
            Some(&token),
            Some(upload_body(12)),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let file_id = upload["file_id"].as_str().unwrap();
        let object_key = upload["object_key"].as_str().unwrap();
        assert!(upload["upload_url"].as_str().unwrap().contains(object_key));

        let (status, body) = request(app.clone(), "GET", "/files", Some(&token), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["files"].as_array().unwrap().len(), 0);

        storage.put_object(object_key, 12);
        let (status, completed) = request(
            app.clone(),
            "POST",
            &format!("/files/{file_id}/complete"),
            Some(&token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(completed["state"], "complete");

        let used: i64 = sqlx::query_scalar("SELECT storage_used_bytes FROM users WHERE email = $1")
            .bind("owner@example.com")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(used, 12);

        let (status, body) = request(
            app.clone(),
            "POST",
            &format!("/files/{file_id}/complete"),
            Some(&token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["error"], "invalid_file_state");

        let used: i64 = sqlx::query_scalar("SELECT storage_used_bytes FROM users WHERE email = $1")
            .bind("owner@example.com")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(used, 12);

        let (status, body) = request(app, "GET", "/files", Some(&token), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["files"].as_array().unwrap().len(), 1);
        assert_eq!(body["files"][0]["id"], file_id);
    }

    #[sqlx::test]
    async fn private_files_do_not_leak_to_other_users(pool: PgPool) {
        let storage = Arc::new(FakeStorage::default());
        let app = crate::app_with_storage(pool, storage.clone());
        let owner_token = signup(app.clone(), "private-owner@example.com").await;
        let other_token = signup(app.clone(), "private-other@example.com").await;

        let (status, upload) = request(
            app.clone(),
            "POST",
            "/files/uploads",
            Some(&owner_token),
            Some(upload_body(9)),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let file_id = upload["file_id"].as_str().unwrap();
        let object_key = upload["object_key"].as_str().unwrap();
        storage.put_object(object_key, 9);
        let (status, _) = request(
            app.clone(),
            "POST",
            &format!("/files/{file_id}/complete"),
            Some(&owner_token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let (status, body) = request(app.clone(), "GET", "/files", Some(&other_token), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["files"].as_array().unwrap().len(), 0);

        let (status, body) = request(
            app.clone(),
            "GET",
            &format!("/files/{file_id}/download"),
            Some(&other_token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "file_not_found");

        let (status, body) = request(
            app,
            "GET",
            &format!("/files/{file_id}/download"),
            Some(&owner_token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(body["download_url"].as_str().unwrap().contains(object_key));
    }
}
