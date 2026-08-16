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

/// read explicit configuration for the mutation-capable live suite
pub fn ami_host() -> String {
    require_live_opt_in();
    required_env("ASTERISK_AMI_HOST")
}

pub fn ami_port() -> u16 {
    require_live_opt_in();
    required_env("ASTERISK_AMI_PORT")
        .parse()
        .expect("ASTERISK_AMI_PORT must be a valid u16")
}

pub fn ari_host() -> String {
    require_live_opt_in();
    required_env("ASTERISK_ARI_HOST")
}

pub fn ari_port() -> u16 {
    require_live_opt_in();
    required_env("ASTERISK_ARI_PORT")
        .parse()
        .expect("ASTERISK_ARI_PORT must be a valid u16")
}

fn require_live_opt_in() {
    assert_eq!(
        std::env::var("ASTERISK_TEST_ALLOW_MUTATION").as_deref(),
        Ok("1"),
        "live tests mutate PBX state; set ASTERISK_TEST_ALLOW_MUTATION=1 only for an isolated test instance"
    );
}

fn required_env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} is required for live tests"))
}
