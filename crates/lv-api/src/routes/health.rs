use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

use crate::state::AppState;

pub async fn healthz(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    match state.storage.ping().await {
        Ok(_) => (StatusCode::OK, Json(json!({ "status": "ok" }))),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "degraded", "detail": "database unreachable" })),
        ),
    }
}
