use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;

use crate::adapters::object_storage::s3::S3ObjectStorage;
use crate::bootstrap::config::{Config, CorsConfig};
use crate::bootstrap::router::build_router;
use crate::bootstrap::state::AppState;

pub async fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=debug".into()),
        )
        .init();

    let config = Config::from_env();
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .expect("failed to connect to postgres");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("failed to run migrations");

    let listener = tokio::net::TcpListener::bind(config.address())
        .await
        .expect("failed to bind address");

    let address = listener
        .local_addr()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|_| config.address());
    let storage = Arc::new(S3ObjectStorage::from_env().expect("S3 storage env vars must be set"));
    let state = AppState::from_parts(
        pool,
        storage,
        config.max_file_size_bytes,
        config.presigned_url_ttl_seconds,
    );

    tracing::info!("drive-clone-api listening on {address}");
    axum::serve(listener, build_router(state, CorsConfig::from_env()))
        .await
        .expect("server crashed");
}
