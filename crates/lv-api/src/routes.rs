use axum::{
    routing::{get, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::state::AppState;

pub mod auth;
pub mod health;
pub mod repos;
pub mod users;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health::healthz))
        // Auth
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/tokens", post(auth::create_token))
        // Users
        .route("/api/v1/users/{username}", get(users::get_user))
        // Repos
        .route("/api/v1/repos", post(repos::create_repo))
        .route("/api/v1/repos/{owner}/{repo}", get(repos::get_repo))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
