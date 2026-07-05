use axum::{extract::State, Json};
use serde_json::Value;

use crate::state::AppState;

pub async fn jwks(State(state): State<AppState>) -> Json<Value> {
    Json(state.jwt.jwks.clone())
}
