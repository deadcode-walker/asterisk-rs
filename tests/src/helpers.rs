use std::sync::OnceLock;

/// initialize tracing for test output (idempotent)
pub fn init_tracing() {
    let _ = tracing_subscriber::fmt::try_init();
}

/// re-raise panics from spawned server tasks so test failures point at the
/// actual panic location instead of producing misleading messages
pub fn assert_server_ok(result: Result<(), tokio::task::JoinError>) {
    match result {
        Ok(()) => {}
        Err(error) if error.is_panic() => std::panic::resume_unwind(error.into_panic()),
        Err(error) => panic!("server task did not complete normally: {error}"),
    }
}

/// Explicit configuration for the mutation-capable live suite.
///
/// `Debug` is deliberately not implemented because this value owns credentials.
pub struct LiveConfig {
    pub ami_host: String,
    pub ami_port: u16,
    pub ami_username: String,
    pub ami_secret: String,
    pub ari_host: String,
    pub ari_port: u16,
    pub ari_username: String,
    pub ari_password: String,
    pub ari_app: String,
    pub instance_marker: String,
    pub run_id: String,
}

impl LiveConfig {
    fn from_env() -> Self {
        assert_eq!(
            std::env::var("ASTERISK_TEST_ALLOW_MUTATION").as_deref(),
            Ok("1"),
            "live tests mutate PBX state; set ASTERISK_TEST_ALLOW_MUTATION=1 only for an isolated test instance"
        );

        let config = Self {
            ami_host: required_env("ASTERISK_AMI_HOST"),
            ami_port: required_port("ASTERISK_AMI_PORT"),
            ami_username: required_env("ASTERISK_AMI_USERNAME"),
            ami_secret: required_env("ASTERISK_AMI_SECRET"),
            ari_host: required_env("ASTERISK_ARI_HOST"),
            ari_port: required_port("ASTERISK_ARI_PORT"),
            ari_username: required_env("ASTERISK_ARI_USERNAME"),
            ari_password: required_env("ASTERISK_ARI_PASSWORD"),
            ari_app: required_env("ASTERISK_ARI_APP"),
            instance_marker: required_env("ASTERISK_TEST_INSTANCE_MARKER"),
            run_id: required_env("ASTERISK_TEST_RUN_ID"),
        };
        assert_safe_component("ASTERISK_TEST_RUN_ID", &config.run_id);
        config
    }

    /// Return a collision-resistant name scoped to this live-test run.
    pub fn resource_name(&self, suffix: &str) -> String {
        assert_safe_component("resource suffix", suffix);
        format!("asterisk-rs-{}-{suffix}", self.run_id)
    }
}

pub fn live_config() -> &'static LiveConfig {
    static CONFIG: OnceLock<LiveConfig> = OnceLock::new();
    CONFIG.get_or_init(LiveConfig::from_env)
}

/// read explicit configuration for the mutation-capable live suite
pub fn ami_host() -> String {
    live_config().ami_host.clone()
}

pub fn ami_port() -> u16 {
    live_config().ami_port
}

pub fn ami_username() -> &'static str {
    &live_config().ami_username
}

pub fn ami_secret() -> &'static str {
    &live_config().ami_secret
}

pub fn ari_host() -> String {
    live_config().ari_host.clone()
}

pub fn ari_port() -> u16 {
    live_config().ari_port
}

pub fn ari_username() -> &'static str {
    &live_config().ari_username
}

pub fn ari_password() -> &'static str {
    &live_config().ari_password
}

pub fn ari_app() -> &'static str {
    &live_config().ari_app
}

fn required_port(name: &str) -> u16 {
    required_env(name)
        .parse()
        .unwrap_or_else(|_| panic!("{name} must be a valid u16"))
}

fn assert_safe_component(name: &str, value: &str) {
    assert!(
        !value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')),
        "{name} must contain 1-64 ASCII letters, digits, '-' or '_'"
    );
}

fn required_env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} is required for live tests"))
}
