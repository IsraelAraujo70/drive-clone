mod auth;
mod files;
mod storage;

use std::env;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::trace::TraceLayer;

use storage::ObjectStorage;

const DEFAULT_MAX_FILE_SIZE_BYTES: i64 = 15 * 1024 * 1024 * 1024;
const DEFAULT_PRESIGNED_URL_TTL_SECONDS: i64 = 900;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub storage: Arc<dyn ObjectStorage>,
    pub max_file_size_bytes: i64,
    pub presigned_url_ttl_seconds: i64,
}

pub fn app(pool: PgPool) -> Router {
    app_with_storage(pool, Arc::new(storage::DisabledStorage))
}

pub fn app_with_storage(pool: PgPool, storage: Arc<dyn ObjectStorage>) -> Router {
    let state = AppState {
        pool,
        storage,
        max_file_size_bytes: env_i64("MAX_FILE_SIZE_BYTES", DEFAULT_MAX_FILE_SIZE_BYTES),
        presigned_url_ttl_seconds: env_i64(
            "PRESIGNED_URL_TTL_SECONDS",
            DEFAULT_PRESIGNED_URL_TTL_SECONDS,
        ),
    };

    app_with_state(state)
}

pub fn app_with_state(state: AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/auth/signup", post(auth::signup))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", get(auth::me))
        .route("/files/uploads", post(files::create_upload))
        .route("/files", get(files::list_files))
        .route("/files/{file_id}/complete", post(files::complete_upload))
        .route("/files/{file_id}/download", get(files::download_file))
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer())
        .with_state(state)
}

fn cors_layer() -> CorsLayer {
    let origin = match env::var("CORS_ALLOWED_ORIGINS") {
        Ok(list) => AllowOrigin::list(list.split(',').filter_map(|o| o.trim().parse().ok())),
        // ponytail: permissive default for local dev; production must set CORS_ALLOWED_ORIGINS
        Err(_) => AllowOrigin::any(),
    };
    CorsLayer::new()
        .allow_origin(origin)
        .allow_methods(Any)
        .allow_headers(Any)
}

async fn root() -> impl IntoResponse {
    Json(serde_json::json!({
        "service": "drive-clone-api",
        "message": "Google Drive clone API",
    }))
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query("SELECT 1").execute(&state.pool).await {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "ok", "service": "drive-clone-api"})),
        ),
        Err(error) => {
            tracing::error!("health check failed: {error}");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"status": "degraded", "service": "drive-clone-api"})),
            )
        }
    }
}

fn env_i64(name: &str, default: i64) -> i64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=debug".into()),
        )
        .init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("failed to connect to postgres");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("failed to run migrations");

    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let address = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .expect("failed to bind address");

    let storage =
        Arc::new(storage::S3Storage::from_env().expect("S3 storage env vars must be set"));

    tracing::info!("drive-clone-api listening on {address}");
    axum::serve(listener, app_with_storage(pool, storage))
        .await
        .expect("server crashed");
}
