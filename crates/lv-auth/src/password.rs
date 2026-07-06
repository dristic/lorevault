use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use crate::error::{AuthError, Result};

pub fn hash(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AuthError::Internal(e.to_string()))
}

pub fn verify(password: &str, hash: &str) -> Result<()> {
    let parsed = PasswordHash::new(hash).map_err(|e| AuthError::Internal(e.to_string()))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| AuthError::InvalidCredentials)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_succeeds() {
        let hash = hash("correct-horse-battery-staple").unwrap();
        assert!(verify("correct-horse-battery-staple", &hash).is_ok());
    }

    #[test]
    fn verify_rejects_wrong_password() {
        let hash = hash("correct-horse-battery-staple").unwrap();
        assert!(matches!(
            verify("wrong-password", &hash),
            Err(AuthError::InvalidCredentials)
        ));
    }

    #[test]
    fn hashes_are_salted_differently() {
        let a = hash("same-password").unwrap();
        let b = hash("same-password").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn verify_rejects_malformed_hash() {
        assert!(verify("anything", "not-a-real-hash").is_err());
    }
}
