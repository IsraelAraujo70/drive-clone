pub mod adapters;
pub mod application;
pub mod bootstrap;
pub mod domain;

use std::sync::Arc;

use axum::Router;
use sqlx::PgPool;

pub use bootstrap::state::AppState;

use adapters::object_storage::disabled::DisabledObjectStorage;
use application::ports::object_storage::ObjectStorage;

pub fn app(pool: PgPool) -> Router {
    app_with_storage(pool, Arc::new(DisabledObjectStorage))
}

pub fn app_with_storage(pool: PgPool, storage: Arc<dyn ObjectStorage>) -> Router {
    let config = bootstrap::config::Config::from_env_defaults();
    let state = bootstrap::state::AppState::from_parts(
        pool,
        storage,
        config.max_file_size_bytes,
        config.presigned_url_ttl_seconds,
    );
    app_with_state(state)
}

pub fn app_with_state(state: AppState) -> Router {
    bootstrap::router::build_router(state, bootstrap::config::CorsConfig::from_env())
}
