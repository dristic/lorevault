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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_tokens_are_unique_and_prefixed() {
        let (raw_a, _) = generate_api_token();
        let (raw_b, _) = generate_api_token();
        assert!(raw_a.starts_with("lv_"));
        assert_ne!(raw_a, raw_b);
    }

    #[test]
    fn hash_api_token_matches_generated_hash() {
        let (raw, hash) = generate_api_token();
        assert_eq!(hash_api_token(&raw), hash);
    }

    #[test]
    fn hash_api_token_is_deterministic() {
        assert_eq!(hash_api_token("lv_abc"), hash_api_token("lv_abc"));
        assert_ne!(hash_api_token("lv_abc"), hash_api_token("lv_xyz"));
    }
}
