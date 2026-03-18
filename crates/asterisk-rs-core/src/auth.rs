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
}
