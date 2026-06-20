use crate::error::{AuthError, Result};

pub fn username(s: &str) -> Result<()> {
    if s.len() < 3 || s.len() > 39 {
        return Err(AuthError::Validation(
            "username must be 3–39 characters".into(),
        ));
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AuthError::Validation(
            "username may only contain letters, numbers, hyphens, and underscores".into(),
        ));
    }
    if s.starts_with(['-', '_']) || s.ends_with(['-', '_']) {
        return Err(AuthError::Validation(
            "username cannot start or end with a hyphen or underscore".into(),
        ));
    }
    Ok(())
}

pub fn email(s: &str) -> Result<()> {
    let (local, domain) = s
        .split_once('@')
        .ok_or_else(|| AuthError::Validation("invalid email address".into()))?;
    if local.is_empty() || !domain.contains('.') || domain.ends_with('.') {
        return Err(AuthError::Validation("invalid email address".into()));
    }
    Ok(())
}

pub fn password(s: &str) -> Result<()> {
    if s.len() < 8 {
        return Err(AuthError::Validation(
            "password must be at least 8 characters".into(),
        ));
    }
    // Argon2 accepts arbitrary length, but very long passwords can be a DoS vector
    if s.len() > 1024 {
        return Err(AuthError::Validation(
            "password must be at most 1024 characters".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_valid() {
        assert!(username("alice").is_ok());
        assert!(username("alice-bob").is_ok());
        assert!(username("alice_bob").is_ok());
        assert!(username("Alice123").is_ok());
    }

    #[test]
    fn username_invalid() {
        assert!(username("ab").is_err());           // too short
        assert!(username("-alice").is_err());        // starts with hyphen
        assert!(username("alice-").is_err());        // ends with hyphen
        assert!(username("alice bob").is_err());     // space
        assert!(username("alice@bob").is_err());     // @
        assert!(username(&"a".repeat(40)).is_err()); // too long
    }

    #[test]
    fn email_valid() {
        assert!(email("user@example.com").is_ok());
        assert!(email("user+tag@sub.example.com").is_ok());
    }

    #[test]
    fn email_invalid() {
        assert!(email("notanemail").is_err());
        assert!(email("@example.com").is_err());
        assert!(email("user@nodot").is_err());
    }

    #[test]
    fn password_valid() {
        assert!(password("correct-horse-battery-staple").is_ok());
        assert!(password("12345678").is_ok());
    }

    #[test]
    fn password_invalid() {
        assert!(password("short").is_err());
        assert!(password(&"a".repeat(1025)).is_err());
    }
}
