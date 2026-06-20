use rand::RngCore;
use sha2::{Digest, Sha256};

/// Generates a cryptographically random API token.
/// Returns `(raw_token, hash)` — store only the hash; send the raw token to the user once.
pub fn generate_api_token() -> (String, String) {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let raw = format!("lv_{}", hex::encode(bytes));
    let hash = hex::encode(Sha256::digest(raw.as_bytes()));
    (raw, hash)
}

/// Hashes a raw API token for database lookup.
pub fn hash_api_token(raw: &str) -> String {
    hex::encode(Sha256::digest(raw.as_bytes()))
}
