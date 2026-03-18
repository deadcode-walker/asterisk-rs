/// initialize tracing for test output (idempotent)
pub fn init_tracing() {
    let _ = tracing_subscriber::fmt::try_init();
}

/// read test config from environment or use defaults
#[allow(dead_code)]
pub fn ami_host() -> String {
    std::env::var("ASTERISK_AMI_HOST").unwrap_or_else(|_| "127.0.0.1".into())
}

#[allow(dead_code)]
pub fn ami_port() -> u16 {
    std::env::var("ASTERISK_AMI_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5038)
}

#[allow(dead_code)]
pub fn ari_host() -> String {
    std::env::var("ASTERISK_ARI_HOST").unwrap_or_else(|_| "127.0.0.1".into())
}

#[allow(dead_code)]
pub fn ari_port() -> u16 {
    std::env::var("ASTERISK_ARI_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8088)
}
