use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use drive_clone_api::adapters::object_storage::fake::FakeObjectStorage;
use drive_clone_api::{AppState, app_with_state, app_with_storage};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

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

fn signup_body(email: &str) -> Value {
    json!({"email": email, "password": "password123", "display_name": "Test User"})
}

async fn signup(app: Router, email: &str) -> String {
    let (status, body) = request(app, "POST", "/auth/signup", None, Some(signup_body(email))).await;
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
async fn signup_creates_account_and_session(pool: PgPool) {
    let app = drive_clone_api::app(pool);
    let (status, body) = request(
        app,
        "POST",
        "/auth/signup",
        None,
        Some(signup_body("New.User@Example.com")),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["user"]["email"], "new.user@example.com");
    assert_eq!(body["user"]["display_name"], "Test User");
    assert_eq!(body["user"]["storage_used_bytes"], 0);
    assert!(body["user"].get("password_hash").is_none());
    assert!(body["token"].as_str().unwrap().len() > 40);
}

#[sqlx::test]
async fn signup_rejects_duplicate_email(pool: PgPool) {
    let app = drive_clone_api::app(pool);
    let (status, _) = request(
        app.clone(),
        "POST",
        "/auth/signup",
        None,
        Some(signup_body("dup@example.com")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, body) = request(
        app,
        "POST",
        "/auth/signup",
        None,
        Some(signup_body("DUP@example.com")),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "email_taken");
}

#[sqlx::test]
async fn signup_rejects_invalid_input(pool: PgPool) {
    let app = drive_clone_api::app(pool);

    let (status, body) = request(
        app.clone(),
        "POST",
        "/auth/signup",
        None,
        Some(json!({"email": "not-an-email", "password": "password123", "display_name": "X"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "validation_error");

    let (status, _) = request(
        app,
        "POST",
        "/auth/signup",
        None,
        Some(json!({"email": "ok@example.com", "password": "short", "display_name": "X"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
async fn login_logout_and_me_follow_session_state(pool: PgPool) {
    let app = drive_clone_api::app(pool);
    let token = signup(app.clone(), "session@example.com").await;

    let (status, body) = request(
        app.clone(),
        "POST",
        "/auth/login",
        None,
        Some(json!({"email": "session@example.com", "password": "password123"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["user"]["email"], "session@example.com");
    let login_token = body["token"].as_str().unwrap();

    let (status, body) = request(app.clone(), "GET", "/auth/me", Some(login_token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["email"], "session@example.com");

    let (status, _) = request(app.clone(), "POST", "/auth/logout", Some(login_token), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, body) = request(app, "GET", "/auth/me", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["email"], "session@example.com");
}

#[sqlx::test]
async fn login_rejects_bad_credentials(pool: PgPool) {
    let app = drive_clone_api::app(pool);
    signup(app.clone(), "bad-login@example.com").await;

    let (status, body) = request(
        app,
        "POST",
        "/auth/login",
        None,
        Some(json!({"email": "bad-login@example.com", "password": "wrong-password"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "invalid_credentials");
}

#[sqlx::test]
async fn health_reports_ok_with_database(pool: PgPool) {
    let app = drive_clone_api::app(pool);
    let (status, body) = request(app, "GET", "/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[sqlx::test]
async fn file_routes_require_auth(pool: PgPool) {
    let storage = Arc::new(FakeObjectStorage::default());
    let app = app_with_storage(pool, storage);

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
    let storage = Arc::new(FakeObjectStorage::default());
    let app = app_with_storage(pool.clone(), storage);
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

    let app = app_with_state(AppState::from_parts(
        pool,
        Arc::new(FakeObjectStorage::default()),
        5,
        900,
    ));
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
    let storage = Arc::new(FakeObjectStorage::default());
    let app = app_with_storage(pool.clone(), storage.clone());
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
    let storage = Arc::new(FakeObjectStorage::default());
    let app = app_with_storage(pool, storage.clone());
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
