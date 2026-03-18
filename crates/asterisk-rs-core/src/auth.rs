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
