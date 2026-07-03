use axum::Router;
use axum::routing::{get, post};
use tower_http::trace::TraceLayer;

use crate::adapters::http::{auth_routes, file_routes};
use crate::bootstrap::config::CorsConfig;
use crate::bootstrap::health::{health, root};
use crate::bootstrap::state::AppState;

pub fn build_router(state: AppState, cors: CorsConfig) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/auth/signup", post(auth_routes::signup))
        .route("/auth/login", post(auth_routes::login))
        .route("/auth/logout", post(auth_routes::logout))
        .route("/auth/me", get(auth_routes::me))
        .route("/files/uploads", post(file_routes::create_upload))
        .route("/files", get(file_routes::list_files))
        .route(
            "/files/{file_id}/complete",
            post(file_routes::complete_upload),
        )
        .route("/files/{file_id}/download", get(file_routes::download_file))
        .layer(TraceLayer::new_for_http())
        .layer(cors.layer())
        .with_state(state)
}
