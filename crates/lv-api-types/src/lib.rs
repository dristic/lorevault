//! Wire-format request/response types for lv-api's REST surface.
//!
//! Shared between `lv-api` (which serves them) and any HTTP client — today
//! that's `lv-cli`. Deliberately depends only on `serde`/`time`/`uuid`, not
//! `lv-core`, so that pulling in these types never drags a server's worth of
//! dependencies (sqlx, axum, tonic, ...) into a thin client.

pub mod admin;
pub mod auth;
pub mod repos;
pub mod users;
