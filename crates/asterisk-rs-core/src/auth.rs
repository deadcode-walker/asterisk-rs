//! Authentication primitives shared across protocols.

/// credentials for connecting to Asterisk
#[derive(Clone)]
pub struct Credentials {
    username: String,
    secret: String,
}

impl Credentials {
    pub fn new(username: impl Into<String>, secret: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            secret: secret.into(),
        }
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn secret(&self) -> &str {
        &self.secret
    }
}

// intentionally omit Debug to avoid leaking secrets in logs
impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("username", &self.username)
            .field("secret", &"[redacted]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_debug_redacts_secret() {
        let creds = Credentials::new("admin", "s3cret");
        let debug = format!("{creds:?}");
        assert!(debug.contains("admin"));
        assert!(debug.contains("[redacted]"));
        assert!(!debug.contains("s3cret"));
    }

    #[test]
    fn credentials_accessors() {
        let creds = Credentials::new("user", "pass");
        assert_eq!(creds.username(), "user");
        assert_eq!(creds.secret(), "pass");
    }

    #[test]
    fn credentials_clone_preserves_values() {
        let creds = Credentials::new("admin", "secret");
        let cloned = creds.clone();
        assert_eq!(cloned.username(), "admin");
        assert_eq!(cloned.secret(), "secret");
    }

    #[test]
    fn credentials_empty_username_and_secret() {
        let creds = Credentials::new("", "");
        assert_eq!(creds.username(), "");
        assert_eq!(creds.secret(), "");
    }

    #[test]
    fn credentials_unicode_values() {
        let creds = Credentials::new("администратор", "密码🔑");
        assert_eq!(creds.username(), "администратор");
        assert_eq!(creds.secret(), "密码🔑");
    }

    #[test]
    fn credentials_special_characters_in_secret() {
        let secret = "line1\nline2:colon spaces\ttab";
        let creds = Credentials::new("user", secret);
        assert_eq!(creds.secret(), secret);
    }

    #[test]
    fn credentials_very_long_strings() {
        let long_user = "u".repeat(10_000);
        let long_secret = "s".repeat(10_000);
        let creds = Credentials::new(long_user.clone(), long_secret.clone());
        assert_eq!(creds.username(), long_user);
        assert_eq!(creds.secret(), long_secret);
    }

    #[test]
    fn credentials_debug_format_structure() {
        let creds = Credentials::new("testuser", "hunter2");
        let debug = format!("{creds:?}");
        // verify the debug output uses debug_struct format
        assert!(debug.starts_with("Credentials {"));
        assert!(debug.contains("username: \"testuser\""));
        assert!(debug.contains("secret: \"[redacted]\""));
        assert!(debug.ends_with("}"));
        // secret value must never appear
        assert!(!debug.contains("hunter2"));
    }
}
