use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::bootstrap::state::AppState;

pub async fn root() -> impl IntoResponse {
    Json(serde_json::json!({
        "service": "drive-clone-api",
        "message": "Google Drive clone API",
    }))
}

pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
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
