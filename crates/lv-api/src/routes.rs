use axum::{
    Router, routing::{get, post}
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::state::AppState;

pub mod auth;
pub mod health;
pub mod repos;
pub mod users;
pub mod well_known;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health::healthz))
        // Well-known public JWKS
        .route("/.well-known/jwks.json", get(well_known::jwks))
        // Browser-based login (Lore CLI device flow)
        .route("/login", get(auth::browser_login_form).post(auth::browser_login_submit))
        // Auth API
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/password", post(auth::change_password))
        .route("/api/v1/auth/password/reset", post(auth::reset_password))
        .route("/api/v1/auth/tokens", post(auth::create_token))
        // Users
        .route("/api/v1/users/{username}", get(users::get_user))
        .route("/api/v1/users/me", get(users::get_me))
        // Repos
        .route("/api/v1/repos/{owner}/{repo}", get(repos::get_repo))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
