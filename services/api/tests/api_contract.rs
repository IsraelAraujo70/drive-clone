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
    upload_body_in_folder(size_bytes, None)
}

fn upload_body_in_folder(size_bytes: i64, parent_folder_id: Option<&str>) -> Value {
    json!({
        "filename": "report.txt",
        "parent_folder_id": parent_folder_id,
        "content_type": "text/plain",
        "size_bytes": size_bytes,
        "checksum_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    })
}

async fn create_folder(
    app: Router,
    token: &str,
    name: &str,
    parent_folder_id: Option<&str>,
) -> String {
    let (status, body) = request(
        app,
        "POST",
        "/folders",
        Some(token),
        Some(json!({"name": name, "parent_folder_id": parent_folder_id})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    body["id"].as_str().unwrap().to_string()
}

async fn create_completed_file(
    app: Router,
    storage: Arc<FakeObjectStorage>,
    token: &str,
    size_bytes: i64,
) -> (String, String) {
    let (status, upload) = request(
        app.clone(),
        "POST",
        "/files/uploads",
        Some(token),
        Some(upload_body(size_bytes)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let file_id = upload["file_id"].as_str().unwrap().to_string();
    let object_key = upload["object_key"].as_str().unwrap().to_string();
    storage.put_object(&object_key, size_bytes);
    let (status, completed) = request(
        app,
        "POST",
        &format!("/files/{file_id}/complete"),
        Some(token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(completed["deleted_at"], Value::Null);
    (file_id, object_key)
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
        ("GET", "/files/trash", None),
        ("GET", "/files/shared-with-me", None),
        ("GET", "/drive", None),
        ("GET", "/drive/trash", None),
        ("GET", "/folders", None),
        (
            "POST",
            "/folders",
            Some(json!({"name": "Projects", "parent_folder_id": null})),
        ),
        (
            "PATCH",
            "/files/11111111-1111-4111-8111-111111111111",
            Some(json!({"filename": "renamed.txt"})),
        ),
        (
            "PATCH",
            "/folders/11111111-1111-4111-8111-111111111111",
            Some(json!({"name": "Renamed"})),
        ),
        (
            "DELETE",
            "/folders/11111111-1111-4111-8111-111111111111",
            None,
        ),
        (
            "POST",
            "/folders/11111111-1111-4111-8111-111111111111/restore",
            None,
        ),
    ] {
        let (status, response) = request(app.clone(), method, uri, None, body).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(response["error"], "unauthorized");
    }
}

#[sqlx::test]
async fn folders_browse_upload_rename_and_move_follow_contract(pool: PgPool) {
    let storage = Arc::new(FakeObjectStorage::default());
    let app = app_with_storage(pool, storage.clone());
    let owner_token = signup(app.clone(), "folders-owner@example.com").await;
    let other_token = signup(app.clone(), "folders-other@example.com").await;

    let projects_id = create_folder(app.clone(), &owner_token, "Projects", None).await;
    let client_id = create_folder(app.clone(), &owner_token, "Client A", Some(&projects_id)).await;

    let (status, root) = request(app.clone(), "GET", "/drive", Some(&owner_token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(root["folders"][0]["id"], projects_id);
    assert_eq!(root["files"].as_array().unwrap().len(), 0);

    let (status, upload) = request(
        app.clone(),
        "POST",
        "/files/uploads",
        Some(&owner_token),
        Some(upload_body_in_folder(12, Some(&projects_id))),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let file_id = upload["file_id"].as_str().unwrap();
    let object_key = upload["object_key"].as_str().unwrap();
    storage.put_object(object_key, 12);
    let (status, completed) = request(
        app.clone(),
        "POST",
        &format!("/files/{file_id}/complete"),
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(completed["parent_folder_id"], projects_id);

    let (status, folder_view) = request(
        app.clone(),
        "GET",
        &format!("/drive?parent_folder_id={projects_id}"),
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(folder_view["breadcrumbs"][0]["id"], projects_id);
    assert_eq!(folder_view["folders"][0]["id"], client_id);
    assert_eq!(folder_view["files"][0]["id"], file_id);

    let (status, renamed_file) = request(
        app.clone(),
        "PATCH",
        &format!("/files/{file_id}"),
        Some(&owner_token),
        Some(json!({"filename": "brief-renamed.txt"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(renamed_file["filename"], "brief-renamed.txt");

    let (status, moved_file) = request(
        app.clone(),
        "PATCH",
        &format!("/files/{file_id}"),
        Some(&owner_token),
        Some(json!({"parent_folder_id": null})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(moved_file["parent_folder_id"], Value::Null);

    let (status, renamed_folder) = request(
        app.clone(),
        "PATCH",
        &format!("/folders/{projects_id}"),
        Some(&owner_token),
        Some(json!({"name": "Work"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(renamed_folder["name"], "Work");

    let (status, body) = request(
        app.clone(),
        "PATCH",
        &format!("/folders/{projects_id}"),
        Some(&owner_token),
        Some(json!({"parent_folder_id": client_id})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "invalid_file_state");

    let (status, body) = request(
        app.clone(),
        "PATCH",
        &format!("/files/{file_id}"),
        Some(&other_token),
        Some(json!({"filename": "stolen.txt"})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "file_not_found");

    let (status, body) = request(
        app,
        "GET",
        &format!("/drive?parent_folder_id={projects_id}"),
        Some(&other_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "file_not_found");
}

#[sqlx::test]
async fn recursive_folder_delete_and_restore_follow_contract(pool: PgPool) {
    let storage = Arc::new(FakeObjectStorage::default());
    let app = app_with_storage(pool, storage.clone());
    let owner_token = signup(app.clone(), "folder-trash-owner@example.com").await;
    let grantee_token = signup(app.clone(), "folder-trash-grantee@example.com").await;

    let work_id = create_folder(app.clone(), &owner_token, "Work", None).await;
    let child_id = create_folder(app.clone(), &owner_token, "Client", Some(&work_id)).await;
    let (status, upload) = request(
        app.clone(),
        "POST",
        "/files/uploads",
        Some(&owner_token),
        Some(upload_body_in_folder(15, Some(&child_id))),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let file_id = upload["file_id"].as_str().unwrap();
    let object_key = upload["object_key"].as_str().unwrap();
    storage.put_object(object_key, 15);
    let (status, _) = request(
        app.clone(),
        "POST",
        &format!("/files/{file_id}/complete"),
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = request(
        app.clone(),
        "POST",
        &format!("/files/{file_id}/shares"),
        Some(&owner_token),
        Some(json!({"email": "folder-trash-grantee@example.com"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, _) = request(
        app.clone(),
        "DELETE",
        &format!("/folders/{work_id}"),
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, body) = request(
        app.clone(),
        "GET",
        &format!("/files/{file_id}/download"),
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "file_not_found");

    let (status, shared) = request(
        app.clone(),
        "GET",
        "/files/shared-with-me",
        Some(&grantee_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(shared["files"].as_array().unwrap().len(), 0);

    let (status, trash) =
        request(app.clone(), "GET", "/drive/trash", Some(&owner_token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(trash["folders"][0]["id"], work_id);
    assert_eq!(trash["files"].as_array().unwrap().len(), 0);

    let (status, restored) = request(
        app.clone(),
        "POST",
        &format!("/folders/{work_id}/restore"),
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(restored["deleted_at"], Value::Null);

    let (status, body) = request(
        app,
        "GET",
        &format!("/files/{file_id}/download"),
        Some(&grantee_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["download_url"].as_str().unwrap().contains(object_key));
}

#[sqlx::test]
async fn soft_delete_restore_and_trash_follow_contract(pool: PgPool) {
    let storage = Arc::new(FakeObjectStorage::default());
    let app = app_with_storage(pool, storage.clone());
    let owner_token = signup(app.clone(), "trash-owner@example.com").await;
    let other_token = signup(app.clone(), "trash-other@example.com").await;
    let (file_id, object_key) = create_completed_file(app.clone(), storage, &owner_token, 12).await;

    let (status, body) = request(
        app.clone(),
        "DELETE",
        &format!("/files/{file_id}"),
        Some(&other_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "file_not_found");

    let (status, body) = request(
        app.clone(),
        "DELETE",
        &format!("/files/{file_id}"),
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(body, Value::Null);

    let (status, body) = request(app.clone(), "GET", "/files", Some(&owner_token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["files"].as_array().unwrap().len(), 0);

    let (status, body) =
        request(app.clone(), "GET", "/files/trash", Some(&owner_token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["files"].as_array().unwrap().len(), 1);
    assert_eq!(body["files"][0]["id"], file_id);
    assert!(body["files"][0]["deleted_at"].as_str().is_some());

    let (status, body) = request(
        app.clone(),
        "GET",
        &format!("/files/{file_id}/download"),
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "file_not_found");

    let (status, restored) = request(
        app.clone(),
        "POST",
        &format!("/files/{file_id}/restore"),
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(restored["deleted_at"], Value::Null);

    let (status, body) = request(
        app,
        "GET",
        &format!("/files/{file_id}/download"),
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["download_url"].as_str().unwrap().contains(&object_key));
}

#[sqlx::test]
async fn shares_allow_grantee_download_and_can_be_listed_or_revoked(pool: PgPool) {
    let storage = Arc::new(FakeObjectStorage::default());
    let app = app_with_storage(pool, storage.clone());
    let owner_token = signup(app.clone(), "share-owner@example.com").await;
    let grantee_token = signup(app.clone(), "share-friend@example.com").await;
    let other_token = signup(app.clone(), "share-other@example.com").await;
    let (file_id, object_key) = create_completed_file(app.clone(), storage, &owner_token, 14).await;

    let (status, share) = request(
        app.clone(),
        "POST",
        &format!("/files/{file_id}/shares"),
        Some(&owner_token),
        Some(json!({"email": "share-friend@example.com"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(share["file_id"], file_id);
    assert_eq!(share["grantee"]["email"], "share-friend@example.com");
    let grantee_id = share["grantee"]["id"].as_str().unwrap();

    let (status, duplicate) = request(
        app.clone(),
        "POST",
        &format!("/files/{file_id}/shares"),
        Some(&owner_token),
        Some(json!({"email": "SHARE-FRIEND@example.com"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(duplicate["created_at"], share["created_at"]);

    let (status, body) = request(
        app.clone(),
        "GET",
        &format!("/files/{file_id}/shares"),
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["shares"].as_array().unwrap().len(), 1);

    let (status, body) = request(
        app.clone(),
        "GET",
        &format!("/files/{file_id}/shares"),
        Some(&other_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "file_not_found");

    let (status, body) = request(
        app.clone(),
        "GET",
        "/files/shared-with-me",
        Some(&grantee_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["files"].as_array().unwrap().len(), 1);
    assert_eq!(body["files"][0]["id"], file_id);
    assert_eq!(
        body["files"][0]["owner"]["email"],
        "share-owner@example.com"
    );

    let (status, body) = request(
        app.clone(),
        "GET",
        &format!("/files/{file_id}/download"),
        Some(&grantee_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["download_url"].as_str().unwrap().contains(&object_key));

    let (status, body) = request(
        app.clone(),
        "DELETE",
        &format!("/files/{file_id}/shares/{grantee_id}"),
        Some(&other_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "file_not_found");

    let (status, _) = request(
        app.clone(),
        "DELETE",
        &format!("/files/{file_id}/shares/{grantee_id}"),
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, body) = request(
        app,
        "GET",
        &format!("/files/{file_id}/download"),
        Some(&grantee_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "file_not_found");
}

#[sqlx::test]
async fn share_rejects_self_unknown_email_and_deleted_files(pool: PgPool) {
    let storage = Arc::new(FakeObjectStorage::default());
    let app = app_with_storage(pool, storage.clone());
    let owner_token = signup(app.clone(), "share-rules-owner@example.com").await;
    signup(app.clone(), "share-rules-friend@example.com").await;
    let (file_id, _) = create_completed_file(app.clone(), storage, &owner_token, 10).await;

    let (status, body) = request(
        app.clone(),
        "POST",
        &format!("/files/{file_id}/shares"),
        Some(&owner_token),
        Some(json!({"email": "share-rules-owner@example.com"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "validation_error");

    let (status, body) = request(
        app.clone(),
        "POST",
        &format!("/files/{file_id}/shares"),
        Some(&owner_token),
        Some(json!({"email": "missing-share-user@example.com"})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "user_not_found");

    let (status, _) = request(
        app.clone(),
        "DELETE",
        &format!("/files/{file_id}"),
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, body) = request(
        app,
        "POST",
        &format!("/files/{file_id}/shares"),
        Some(&owner_token),
        Some(json!({"email": "share-rules-friend@example.com"})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "file_not_found");
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
