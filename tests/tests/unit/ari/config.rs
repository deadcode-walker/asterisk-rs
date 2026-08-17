#![allow(clippy::unwrap_used)]

use asterisk_rs_ari::AriClient;
use asterisk_rs_ari::config::{
    AriConfigBuilder, DEFAULT_MAX_RESPONSE_BODY_BYTES, DEFAULT_MAX_WEBSOCKET_MESSAGE_BYTES,
    DEFAULT_REQUEST_TIMEOUT, TransportMode,
};
use asterisk_rs_ari::error::AriError;
use asterisk_rs_core::config::ReconnectPolicy;
use std::time::Duration;

// ── config tests (12 migrated) ──────────────────────────────────────────────

#[test]
fn build_default_config() {
    let config = AriConfigBuilder::new("myapp")
        .username("admin")
        .password("secret")
        .build()
        .expect("default config should build");

    assert_eq!(config.base_url().as_str(), "http://127.0.0.1:8088/ari");
    assert_eq!(config.request_timeout(), DEFAULT_REQUEST_TIMEOUT);
    assert_eq!(
        config.max_response_body_bytes(),
        DEFAULT_MAX_RESPONSE_BODY_BYTES
    );
    assert_eq!(
        config.max_websocket_message_bytes(),
        DEFAULT_MAX_WEBSOCKET_MESSAGE_BYTES
    );
}

#[test]
fn build_custom_request_limits() {
    let config = AriConfigBuilder::new("myapp")
        .username("admin")
        .password("secret")
        .request_timeout(Duration::from_millis(750))
        .max_response_body_bytes(8192)
        .max_websocket_message_bytes(16384)
        .build()
        .expect("custom limits should build");

    assert_eq!(config.request_timeout(), Duration::from_millis(750));
    assert_eq!(config.max_response_body_bytes(), 8192);
    assert_eq!(config.max_websocket_message_bytes(), 16384);
}

#[test]
fn zero_request_timeout_is_rejected() {
    let error = AriConfigBuilder::new("myapp")
        .username("admin")
        .password("secret")
        .request_timeout(Duration::ZERO)
        .build()
        .expect_err("zero request timeout must be rejected");

    assert!(matches!(error, AriError::InvalidConfig(_)));
}

#[test]
fn unrepresentable_request_timeout_is_rejected() {
    let error = AriConfigBuilder::new("myapp")
        .username("admin")
        .password("secret")
        .request_timeout(Duration::MAX)
        .build()
        .expect_err("unrepresentable request timeout must be rejected");

    assert!(matches!(error, AriError::InvalidConfig(_)));
}

#[test]
fn zero_response_body_limit_is_rejected() {
    let error = AriConfigBuilder::new("myapp")
        .username("admin")
        .password("secret")
        .max_response_body_bytes(0)
        .build()
        .expect_err("zero response body limit must be rejected");

    assert!(matches!(error, AriError::InvalidConfig(_)));
}

#[test]
fn zero_websocket_message_limit_is_rejected() {
    let error = AriConfigBuilder::new("myapp")
        .username("admin")
        .password("secret")
        .max_websocket_message_bytes(0)
        .build()
        .expect_err("zero WebSocket message limit must be rejected");

    assert!(matches!(error, AriError::InvalidConfig(_)));
}

#[tokio::test]
async fn client_transport_modes_construct_explicit_tls_configuration() {
    for mode in [TransportMode::Http, TransportMode::WebSocket] {
        let config = AriConfigBuilder::new("myapp")
            .username("admin")
            .password("secret")
            .secure(true)
            .transport(mode)
            .reconnect(ReconnectPolicy::none())
            .build()
            .expect("config should build");
        let result = AriClient::connect(config).await;
        assert!(
            matches!(result, Err(AriError::WebSocket(_))),
            "TLS configuration must construct before the expected unreachable-server failure: {result:?}"
        );
    }
}

#[test]
fn build_with_custom_host_port() {
    let config = AriConfigBuilder::new("myapp")
        .host("10.0.0.1")
        .port(9999)
        .allow_insecure_remote(true)
        .username("admin")
        .password("secret")
        .build()
        .expect("custom host/port should build");

    assert!(
        config.base_url().as_str().contains("10.0.0.1:9999"),
        "base_url should contain custom host:port, got: {}",
        config.base_url()
    );
}

#[test]
fn remote_cleartext_requires_explicit_opt_in() {
    let error = AriConfigBuilder::new("myapp")
        .host("192.0.2.10")
        .username("admin")
        .password("secret")
        .build()
        .expect_err("remote cleartext must be rejected by default");
    assert!(
        matches!(error, AriError::InvalidConfig(message) if message.contains("allow_insecure_remote"))
    );

    AriConfigBuilder::new("myapp")
        .host("192.0.2.10")
        .username("admin")
        .password("secret")
        .allow_insecure_remote(true)
        .build()
        .expect("explicit remote cleartext opt-in should build");
}

#[test]
fn loopback_cleartext_remains_allowed() {
    for host in ["localhost", "127.0.0.1", "::1"] {
        AriConfigBuilder::new("myapp")
            .host(host)
            .username("admin")
            .password("secret")
            .build()
            .unwrap_or_else(|error| panic!("loopback {host} should build: {error}"));
    }
}

#[test]
fn malformed_private_ca_is_rejected_during_build() {
    let error = AriConfigBuilder::new("myapp")
        .secure(true)
        .username("admin")
        .password("secret")
        .private_ca_pem(b"not a certificate".to_vec())
        .build()
        .expect_err("malformed CA must fail before connection");
    assert!(matches!(error, AriError::InvalidConfig(message) if message.contains("private CA")));
}

#[test]
fn build_secure_uses_https_wss() {
    let config = AriConfigBuilder::new("myapp")
        .secure(true)
        .username("admin")
        .password("secret")
        .build()
        .expect("secure config should build");

    assert!(
        config.base_url().as_str().starts_with("https://"),
        "base_url should use https, got: {}",
        config.base_url()
    );
}

#[test]
fn build_empty_app_name_fails() {
    let err = AriConfigBuilder::new("")
        .username("admin")
        .password("secret")
        .build()
        .expect_err("empty app_name via constructor should fail");

    match err {
        AriError::InvalidUrl(msg) => {
            assert!(
                msg.contains("app_name"),
                "error should mention app_name: {msg}"
            );
        }
        other => panic!("expected InvalidUrl, got: {other:?}"),
    }
}

#[test]
fn build_empty_app_name_via_setter_fails() {
    let err = AriConfigBuilder::new("valid")
        .app_name("")
        .username("admin")
        .password("secret")
        .build()
        .expect_err("empty app_name via setter should fail");

    match err {
        AriError::InvalidUrl(msg) => {
            assert!(
                msg.contains("app_name"),
                "error should mention app_name: {msg}"
            );
        }
        other => panic!("expected InvalidUrl, got: {other:?}"),
    }
}

#[test]
fn config_preserves_app_name() {
    let config = AriConfigBuilder::new("test_app")
        .username("admin")
        .password("secret")
        .build()
        .expect("config should build");

    assert_eq!(config.app_name(), "test_app");
}

#[test]
fn config_preserves_credentials() {
    let config = AriConfigBuilder::new("myapp")
        .username("admin")
        .password("secret")
        .build()
        .expect("config with credentials should build");

    assert_eq!(config.credentials().username(), "admin");
    assert_eq!(config.credentials().secret(), "secret");
}

#[test]
fn build_with_custom_reconnect_policy() {
    let policy = ReconnectPolicy::fixed(Duration::from_secs(5));

    let config = AriConfigBuilder::new("myapp")
        .reconnect(policy)
        .username("admin")
        .password("secret")
        .build()
        .expect("config with reconnect policy should build");

    assert_eq!(
        config.reconnect_policy().initial_delay,
        Duration::from_secs(5)
    );
    assert_eq!(config.reconnect_policy().max_delay, Duration::from_secs(5));
}

#[test]
fn config_fields_accessible() {
    let config = AriConfigBuilder::new("myapp")
        .host("asterisk.local")
        .port(5080)
        .username("user1")
        .password("pass1")
        .secure(true)
        .build()
        .expect("full config should build");

    assert_eq!(config.app_name(), "myapp");
    assert_eq!(config.credentials().username(), "user1");
    assert_eq!(config.credentials().secret(), "pass1");
    assert_eq!(
        config.base_url().as_str(),
        "https://asterisk.local:5080/ari"
    );
    // reconnect_policy is accessible (default)
    let _ = config.reconnect_policy();
}

#[test]
fn builder_fluent_chain() {
    // all builder methods return Self, so they can be chained in a single expression
    let result = AriConfigBuilder::new("chain")
        .host("localhost")
        .port(8088)
        .username("u")
        .password("p")
        .app_name("chain2")
        .secure(false)
        .reconnect(ReconnectPolicy::default())
        .request_timeout(Duration::from_secs(2))
        .max_response_body_bytes(1024)
        .max_websocket_message_bytes(2048)
        .build();

    assert!(result.is_ok(), "fluent chain should produce valid config");
}

#[test]
fn default_host_is_localhost() {
    let config = AriConfigBuilder::new("myapp")
        .username("admin")
        .password("secret")
        .build()
        .expect("default config should build");

    assert!(
        config.base_url().as_str().contains("127.0.0.1"),
        "default host should be 127.0.0.1, got: {}",
        config.base_url()
    );
}

#[test]
fn default_port_is_8088() {
    let config = AriConfigBuilder::new("myapp")
        .username("admin")
        .password("secret")
        .build()
        .expect("default config should build");

    assert!(
        config.base_url().as_str().contains(":8088"),
        "default port should be 8088, got: {}",
        config.base_url()
    );
}
