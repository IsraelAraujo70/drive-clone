use std::sync::OnceLock;

use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::Json;
use axum::extract::{FromRequestParts, State};
use axum::http::StatusCode;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::AppState;

const SESSION_TTL_DAYS: i64 = 30;

const USER_COLUMNS: &str =
    "id, email, display_name, storage_quota_bytes, storage_used_bytes, created_at";

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub storage_quota_bytes: i64,
    pub storage_used_bytes: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct UserWithPassword {
    #[sqlx(flatten)]
    user: User,
    password_hash: String,
}

#[derive(Debug)]
pub enum ApiError {
    Validation(&'static str),
    EmailTaken,
    InvalidCredentials,
    Unauthorized,
    QuotaExceeded,
    FileTooLarge,
    FileNotFound,
    InvalidFileState,
    StorageError,
    Internal,
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        tracing::error!("database error: {error}");
        ApiError::Internal
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            ApiError::Validation(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_error",
                message,
            ),
            ApiError::EmailTaken => (
                StatusCode::CONFLICT,
                "email_taken",
                "An account with this email already exists",
            ),
            ApiError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                "Invalid email or password",
            ),
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Missing or invalid session token",
            ),
            ApiError::QuotaExceeded => (
                StatusCode::CONFLICT,
                "quota_exceeded",
                "Not enough storage quota is available",
            ),
            ApiError::FileTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "file_too_large",
                "File exceeds the maximum allowed size",
            ),
            ApiError::FileNotFound => (
                StatusCode::NOT_FOUND,
                "file_not_found",
                "File was not found",
            ),
            ApiError::InvalidFileState => (
                StatusCode::CONFLICT,
                "invalid_file_state",
                "File is not in the expected state",
            ),
            ApiError::StorageError => (
                StatusCode::BAD_GATEWAY,
                "storage_error",
                "Object storage could not satisfy the request",
            ),
            ApiError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "Something went wrong",
            ),
        };

        (status, Json(json!({"error": code, "message": message}))).into_response()
    }
}

#[derive(Deserialize)]
pub struct SignupRequest {
    email: String,
    password: String,
    display_name: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    user: User,
    token: String,
}

pub struct Auth {
    pub user: User,
    pub token_hash: String,
}

impl FromRequestParts<AppState> for Auth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or(ApiError::Unauthorized)?;

        let token_hash = hash_token(token);
        let query = format!(
            "SELECT {} FROM sessions s JOIN users u ON u.id = s.user_id \
             WHERE s.token_hash = $1 AND s.expires_at > now()",
            USER_COLUMNS
                .split(", ")
                .map(|column| format!("u.{column}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let user = sqlx::query_as::<_, User>(&query)
            .bind(&token_hash)
            .fetch_optional(&state.pool)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        Ok(Auth { user, token_hash })
    }
}

pub async fn signup(
    State(state): State<AppState>,
    Json(request): Json<SignupRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let email = request.email.trim().to_lowercase();
    let display_name = request.display_name.trim().to_string();
    validate_email(&email)?;
    validate_password(&request.password)?;
    validate_display_name(&display_name)?;

    let password = request.password;
    let password_hash = tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(|_| ApiError::Internal)??;

    let query = format!(
        "INSERT INTO users (email, password_hash, display_name) VALUES ($1, $2, $3) RETURNING {USER_COLUMNS}"
    );
    let user = match sqlx::query_as::<_, User>(&query)
        .bind(&email)
        .bind(&password_hash)
        .bind(&display_name)
        .fetch_one(&state.pool)
        .await
    {
        Ok(user) => user,
        Err(sqlx::Error::Database(db)) if db.is_unique_violation() => {
            return Err(ApiError::EmailTaken);
        }
        Err(error) => return Err(error.into()),
    };

    let token = create_session(&state.pool, user.id).await?;
    Ok((StatusCode::CREATED, Json(AuthResponse { user, token })))
}

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let email = request.email.trim().to_lowercase();
    let query = format!("SELECT {USER_COLUMNS}, password_hash FROM users WHERE email = $1");
    let row = sqlx::query_as::<_, UserWithPassword>(&query)
        .bind(&email)
        .fetch_optional(&state.pool)
        .await?;

    let password = request.password;
    let (user, verified) = match row {
        Some(row) => {
            let password_hash = row.password_hash;
            let verified =
                tokio::task::spawn_blocking(move || verify_password(&password, &password_hash))
                    .await
                    .map_err(|_| ApiError::Internal)?;
            (Some(row.user), verified)
        }
        None => {
            // Burn the same hashing cost for unknown emails so response timing
            // does not reveal whether an account exists.
            tokio::task::spawn_blocking(move || verify_password(&password, dummy_hash()))
                .await
                .map_err(|_| ApiError::Internal)?;
            (None, false)
        }
    };

    let user = user
        .filter(|_| verified)
        .ok_or(ApiError::InvalidCredentials)?;
    let token = create_session(&state.pool, user.id).await?;
    Ok((StatusCode::OK, Json(AuthResponse { user, token })))
}

pub async fn logout(State(state): State<AppState>, auth: Auth) -> Result<StatusCode, ApiError> {
    sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(&auth.token_hash)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn me(auth: Auth) -> Json<User> {
    Json(auth.user)
}

async fn create_session(pool: &PgPool, user_id: Uuid) -> Result<String, ApiError> {
    // ponytail: expired rows are only filtered at read time; a cleanup job
    // arrives with the worker service milestone.
    let token = generate_token();
    let expires_at = Utc::now() + Duration::days(SESSION_TTL_DAYS);
    sqlx::query("INSERT INTO sessions (token_hash, user_id, expires_at) VALUES ($1, $2, $3)")
        .bind(hash_token(&token))
        .bind(user_id)
        .bind(expires_at)
        .execute(pool)
        .await?;
    Ok(token)
}

fn generate_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn hash_token(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| ApiError::Internal)
}

fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

fn dummy_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| hash_password("not-a-real-password").expect("hashing cannot fail"))
}

fn validate_email(email: &str) -> Result<(), ApiError> {
    let valid = email.len() <= 254
        && !email.contains(char::is_whitespace)
        && match email.split_once('@') {
            Some((local, domain)) => {
                !local.is_empty()
                    && !domain.contains('@')
                    && domain.contains('.')
                    && !domain.starts_with('.')
                    && !domain.ends_with('.')
            }
            None => false,
        };

    if valid {
        Ok(())
    } else {
        Err(ApiError::Validation("Enter a valid email address"))
    }
}

fn validate_password(password: &str) -> Result<(), ApiError> {
    if password.chars().count() >= 8 && password.len() <= 128 {
        Ok(())
    } else {
        Err(ApiError::Validation(
            "Password must be between 8 and 128 characters",
        ))
    }
}

fn validate_display_name(display_name: &str) -> Result<(), ApiError> {
    if !display_name.is_empty() && display_name.chars().count() <= 100 {
        Ok(())
    } else {
        Err(ApiError::Validation(
            "Name must be between 1 and 100 characters",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;

    #[test]
    fn email_validation_accepts_normal_addresses() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("first.last@sub.domain.dev").is_ok());
    }

    #[test]
    fn email_validation_rejects_malformed_addresses() {
        for email in [
            "",
            "no-at-sign",
            "@example.com",
            "user@",
            "user@nodot",
            "user@.com",
            "user@domain.",
            "a b@example.com",
        ] {
            assert!(validate_email(email).is_err(), "should reject {email:?}");
        }
    }

    #[test]
    fn password_validation_enforces_length() {
        assert!(validate_password("12345678").is_ok());
        assert!(validate_password("1234567").is_err());
        assert!(validate_password(&"x".repeat(129)).is_err());
    }

    #[test]
    fn tokens_are_random_and_hash_deterministically() {
        let first = generate_token();
        let second = generate_token();
        assert_ne!(first, second);
        assert_eq!(first.len(), 43); // 32 bytes, base64url, no padding
        assert_eq!(hash_token(&first), hash_token(&first));
        assert_eq!(hash_token(&first).len(), 64); // sha-256 hex
    }

    #[test]
    fn password_hashing_round_trips() {
        let hash = hash_password("hunter2hunter2").unwrap();
        assert!(verify_password("hunter2hunter2", &hash));
        assert!(!verify_password("wrong-password", &hash));
        assert!(!verify_password("hunter2hunter2", "not-a-phc-string"));
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

    fn signup_body(email: &str) -> Value {
        json!({"email": email, "password": "password123", "display_name": "Test User"})
    }

    #[sqlx::test]
    async fn signup_creates_account_and_session(pool: PgPool) {
        let app = crate::app(pool);
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
        let app = crate::app(pool);
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
        let app = crate::app(pool);

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
    async fn login_succeeds_with_correct_password(pool: PgPool) {
        let app = crate::app(pool);
        request(
            app.clone(),
            "POST",
            "/auth/signup",
            None,
            Some(signup_body("login@example.com")),
        )
        .await;

        let (status, body) = request(
            app,
            "POST",
            "/auth/login",
            None,
            Some(json!({"email": "Login@Example.com", "password": "password123"})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["user"]["email"], "login@example.com");
        assert!(body["token"].as_str().unwrap().len() > 40);
    }

    #[sqlx::test]
    async fn login_rejects_bad_credentials(pool: PgPool) {
        let app = crate::app(pool);
        request(
            app.clone(),
            "POST",
            "/auth/signup",
            None,
            Some(signup_body("victim@example.com")),
        )
        .await;

        let (status, body) = request(
            app.clone(),
            "POST",
            "/auth/login",
            None,
            Some(json!({"email": "victim@example.com", "password": "wrong-password"})),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["error"], "invalid_credentials");

        let (status, body) = request(
            app,
            "POST",
            "/auth/login",
            None,
            Some(json!({"email": "ghost@example.com", "password": "password123"})),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["error"], "invalid_credentials");
    }

    #[sqlx::test]
    async fn me_requires_a_valid_session(pool: PgPool) {
        let app = crate::app(pool);
        let (_, signup) = request(
            app.clone(),
            "POST",
            "/auth/signup",
            None,
            Some(signup_body("me@example.com")),
        )
        .await;
        let token = signup["token"].as_str().unwrap();

        let (status, body) = request(app.clone(), "GET", "/auth/me", Some(token), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["email"], "me@example.com");

        let (status, _) = request(app.clone(), "GET", "/auth/me", None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        let (status, _) = request(app, "GET", "/auth/me", Some("forged-token"), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn expired_sessions_are_rejected(pool: PgPool) {
        let app = crate::app(pool.clone());
        let (_, signup) = request(
            app.clone(),
            "POST",
            "/auth/signup",
            None,
            Some(signup_body("expired@example.com")),
        )
        .await;
        let user_id = Uuid::parse_str(signup["user"]["id"].as_str().unwrap()).unwrap();

        let expired_token = generate_token();
        sqlx::query(
            "INSERT INTO sessions (token_hash, user_id, expires_at) VALUES ($1, $2, now() - interval '1 day')",
        )
        .bind(hash_token(&expired_token))
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();

        let (status, _) = request(app, "GET", "/auth/me", Some(&expired_token), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn logout_revokes_the_session(pool: PgPool) {
        let app = crate::app(pool);
        let (_, signup) = request(
            app.clone(),
            "POST",
            "/auth/signup",
            None,
            Some(signup_body("bye@example.com")),
        )
        .await;
        let token = signup["token"].as_str().unwrap();

        let (status, _) = request(app.clone(), "POST", "/auth/logout", Some(token), None).await;
        assert_eq!(status, StatusCode::NO_CONTENT);

        let (status, _) = request(app, "GET", "/auth/me", Some(token), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn health_reports_ok_with_database(pool: PgPool) {
        let app = crate::app(pool);
        let (status, body) = request(app, "GET", "/health", None, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ok");
    }
}
