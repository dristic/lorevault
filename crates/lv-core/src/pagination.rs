//! Keyset ("cursor") pagination shared by every paginated `Storage` listing.
//!
//! Cursors are opaque, base64-encoded JSON blobs of the last-seen row's sort
//! key(s) — callers never construct or parse one, they just pass back
//! whatever `next_cursor` a previous page returned.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{de::DeserializeOwned, Serialize};

use crate::error::{CoreError, Result};

pub const DEFAULT_LIMIT: u32 = 50;
pub const MAX_LIMIT: u32 = 200;

/// Clamps a client-supplied page size into `[1, MAX_LIMIT]`, defaulting to
/// `DEFAULT_LIMIT` when the client didn't ask for one.
pub fn clamp_limit(limit: Option<u32>) -> u32 {
    limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
}

pub fn encode_cursor<T: Serialize>(key: &T) -> String {
    let json = serde_json::to_vec(key).expect("cursor key is always serializable");
    URL_SAFE_NO_PAD.encode(json)
}

pub fn decode_cursor<T: DeserializeOwned>(cursor: &str) -> Result<T> {
    let bytes = URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| CoreError::Validation("invalid pagination cursor".into()))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| CoreError::Validation("invalid pagination cursor".into()))
}

/// One page of a keyset-paginated listing. `next_cursor` is `Some` iff more
/// rows exist past `items`.
#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}
