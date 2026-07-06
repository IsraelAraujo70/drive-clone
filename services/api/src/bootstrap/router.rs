use axum::Router;
use axum::routing::{delete, get, patch, post};
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
        .route(
            "/folders",
            post(file_routes::create_folder).get(file_routes::list_folders),
        )
        .route("/drive", get(file_routes::browse_drive))
        .route("/search", get(file_routes::search_files))
        .route("/sync/changes", get(file_routes::list_sync_changes))
        .route("/drive/trash", get(file_routes::list_drive_trash))
        .route("/files/uploads", post(file_routes::create_upload))
        .route(
            "/files/uploads/resumable",
            post(file_routes::create_resumable_upload),
        )
        .route(
            "/files/uploads/cleanup-expired",
            post(file_routes::expire_resumable_uploads),
        )
        .route(
            "/files/uploads/{file_id}/status",
            get(file_routes::get_upload_status),
        )
        .route(
            "/files/uploads/{file_id}/parts",
            post(file_routes::presign_upload_part),
        )
        .route(
            "/files/uploads/{file_id}/parts/{part_number}",
            post(file_routes::record_upload_part),
        )
        .route(
            "/files/uploads/{file_id}/finalize",
            post(file_routes::finalize_resumable_upload),
        )
        .route("/files", get(file_routes::list_files))
        .route("/files/trash", get(file_routes::list_trash))
        .route(
            "/files/shared-with-me",
            get(file_routes::list_shared_with_me),
        )
        .route(
            "/files/{file_id}/complete",
            post(file_routes::complete_upload),
        )
        .route("/files/{file_id}/download", get(file_routes::download_file))
        .route(
            "/files/{file_id}",
            delete(file_routes::delete_file).patch(file_routes::update_file),
        )
        .route("/files/{file_id}/restore", post(file_routes::restore_file))
        .route(
            "/folders/{folder_id}",
            patch(file_routes::update_folder).delete(file_routes::delete_folder),
        )
        .route(
            "/folders/{folder_id}/restore",
            post(file_routes::restore_folder),
        )
        .route(
            "/files/{file_id}/shares",
            post(file_routes::share_file).get(file_routes::list_shares),
        )
        .route(
            "/files/{file_id}/shares/{grantee_id}",
            delete(file_routes::revoke_share),
        )
        .layer(TraceLayer::new_for_http())
        .layer(cors.layer())
        .with_state(state)
}
