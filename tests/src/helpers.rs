/// initialize tracing for test output (idempotent)
pub fn init_tracing() {
    let _ = tracing_subscriber::fmt::try_init();
}

/// re-raise panics from spawned server tasks so test failures point at the
/// actual panic location instead of producing misleading messages
pub fn assert_server_ok(result: Result<(), tokio::task::JoinError>) {
    if let Err(e) = result {
        if e.is_panic() {
            std::panic::resume_unwind(e.into_panic());
        }
    }
}

/// read test config from environment or use defaults
pub fn ami_host() -> String {
    std::env::var("ASTERISK_AMI_HOST").unwrap_or_else(|_| "127.0.0.1".into())
}

pub fn ami_port() -> u16 {
    std::env::var("ASTERISK_AMI_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5038)
}

pub fn ari_host() -> String {
    std::env::var("ASTERISK_ARI_HOST").unwrap_or_else(|_| "127.0.0.1".into())
}

pub fn ari_port() -> u16 {
    std::env::var("ASTERISK_ARI_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8088)
}
