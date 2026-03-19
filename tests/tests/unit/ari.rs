#![allow(clippy::unwrap_used)]

use asterisk_rs_ari::client::url_encode;
use asterisk_rs_ari::config::AriConfigBuilder;
use asterisk_rs_ari::error::AriError;
use asterisk_rs_ari::event::{
    AriEvent, AriMessage, Bridge, CallerId, Channel, ContactInfo, DeviceState, DialplanCep,
    Endpoint, LiveRecording, Peer, Playback, ReferTo, ReferredBy, TextMessage,
};
use asterisk_rs_ari::resources::application::Application;
use asterisk_rs_ari::resources::asterisk::{
    AsteriskInfo, AsteriskPing, ConfigTuple, LogChannel, Module, Variable as AsteriskVariable,
};
use asterisk_rs_ari::resources::channel::{OriginateParams, Variable as ChannelVariable};
use asterisk_rs_ari::resources::device_state::DeviceState as ResourceDeviceState;
use asterisk_rs_ari::resources::endpoint::Endpoint as ResourceEndpoint;
use asterisk_rs_ari::resources::mailbox::Mailbox;
use asterisk_rs_ari::resources::recording::StoredRecording;
use asterisk_rs_ari::resources::sound::{Sound, SoundFormat};
use asterisk_rs_core::config::ReconnectPolicy;
use asterisk_rs_core::error::{AuthError, ConnectionError};
use std::time::Duration;

// ── config tests (12 migrated) ──────────────────────────────────────────────

#[test]
fn build_default_config() {
    let config = AriConfigBuilder::new("myapp")
        .build()
        .expect("default config should build");

    assert_eq!(config.base_url.as_str(), "http://127.0.0.1:8088/ari");
    assert!(
        config.ws_url.as_str().starts_with("ws://"),
        "ws_url should start with ws://, got: {}",
        config.ws_url
    );
}

#[test]
fn build_with_custom_host_port() {
    let config = AriConfigBuilder::new("myapp")
        .host("10.0.0.1")
        .port(9999)
        .build()
        .expect("custom host/port should build");

    assert!(
        config.base_url.as_str().contains("10.0.0.1:9999"),
        "base_url should contain custom host:port, got: {}",
        config.base_url
    );
    assert!(
        config.ws_url.as_str().contains("10.0.0.1:9999"),
        "ws_url should contain custom host:port, got: {}",
        config.ws_url
    );
}

#[test]
fn build_secure_uses_https_wss() {
    let config = AriConfigBuilder::new("myapp")
        .secure(true)
        .build()
        .expect("secure config should build");

    assert!(
        config.base_url.as_str().starts_with("https://"),
        "base_url should use https, got: {}",
        config.base_url
    );
    assert!(
        config.ws_url.as_str().starts_with("wss://"),
        "ws_url should use wss, got: {}",
        config.ws_url
    );
}

#[test]
fn build_empty_app_name_fails() {
    let err = AriConfigBuilder::new("")
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
fn ws_url_contains_app_name() {
    let config = AriConfigBuilder::new("test_app")
        .build()
        .expect("config should build");

    assert!(
        config.ws_url.as_str().contains("app=test_app"),
        "ws_url should contain app=test_app, got: {}",
        config.ws_url
    );
}

#[test]
fn ws_url_contains_credentials() {
    let config = AriConfigBuilder::new("myapp")
        .username("admin")
        .password("secret")
        .build()
        .expect("config with credentials should build");

    assert!(
        config.ws_url.as_str().contains("api_key=admin:secret"),
        "ws_url should contain api_key=admin:secret, got: {}",
        config.ws_url
    );
}

#[test]
fn build_with_custom_reconnect_policy() {
    let policy = ReconnectPolicy::fixed(Duration::from_secs(5));

    let config = AriConfigBuilder::new("myapp")
        .reconnect(policy)
        .build()
        .expect("config with reconnect policy should build");

    assert_eq!(
        config.reconnect_policy.initial_delay,
        Duration::from_secs(5)
    );
    assert_eq!(config.reconnect_policy.max_delay, Duration::from_secs(5));
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

    assert_eq!(config.app_name, "myapp");
    assert_eq!(config.username, "user1");
    assert_eq!(config.password, "pass1");
    assert_eq!(config.base_url.as_str(), "https://asterisk.local:5080/ari");
    assert!(config.ws_url.as_str().starts_with("wss://"));
    // reconnect_policy is accessible (default)
    let _ = &config.reconnect_policy;
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
        .build();

    assert!(result.is_ok(), "fluent chain should produce valid config");
}

#[test]
fn default_host_is_localhost() {
    let config = AriConfigBuilder::new("myapp")
        .build()
        .expect("default config should build");

    assert!(
        config.base_url.as_str().contains("127.0.0.1"),
        "default host should be 127.0.0.1, got: {}",
        config.base_url
    );
}

#[test]
fn default_port_is_8088() {
    let config = AriConfigBuilder::new("myapp")
        .build()
        .expect("default config should build");

    assert!(
        config.base_url.as_str().contains(":8088"),
        "default port should be 8088, got: {}",
        config.base_url
    );
}

// ── url_encode tests (10 migrated) ──────────────────────────────────────────

#[test]
fn url_encode_preserves_unreserved() {
    assert_eq!(url_encode("abcXYZ019"), "abcXYZ019");
    assert_eq!(url_encode("-_."), "-_.");
    assert_eq!(url_encode("~"), "~");
}

#[test]
fn url_encode_encodes_spaces() {
    assert_eq!(url_encode("hello world"), "hello%20world");
}

#[test]
fn url_encode_encodes_special_chars() {
    assert_eq!(url_encode("/"), "%2F");
    assert_eq!(url_encode("?"), "%3F");
    assert_eq!(url_encode("&"), "%26");
    assert_eq!(url_encode("="), "%3D");
    assert_eq!(url_encode("#"), "%23");
    assert_eq!(url_encode("@"), "%40");
    assert_eq!(url_encode(":"), "%3A");
    assert_eq!(url_encode("+"), "%2B");
    assert_eq!(url_encode("!"), "%21");
    assert_eq!(url_encode("$"), "%24");
    assert_eq!(url_encode(","), "%2C");
}

#[test]
fn url_encode_empty_string() {
    assert_eq!(url_encode(""), "");
}

#[test]
fn url_encode_unicode() {
    // é is U+00E9, encoded as 0xC3 0xA9 in UTF-8
    assert_eq!(url_encode("é"), "%C3%A9");
    // 日 is U+65E5, encoded as 0xE6 0x97 0xA5 in UTF-8
    assert_eq!(url_encode("日"), "%E6%97%A5");
}

#[test]
fn url_encode_already_encoded() {
    // % itself must be encoded, so %20 in input becomes %2520
    assert_eq!(url_encode("%20"), "%2520");
}

#[test]
fn url_encode_slash() {
    assert_eq!(url_encode("a/b"), "a%2Fb");
}

#[test]
fn url_encode_all_unreserved_chars() {
    let unreserved = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.~";
    assert_eq!(url_encode(unreserved), unreserved);
}

#[test]
fn url_encode_mixed_content() {
    assert_eq!(url_encode("hello world/foo"), "hello%20world%2Ffoo");
}

#[test]
fn url_encode_parentheses() {
    assert_eq!(url_encode("("), "%28");
    assert_eq!(url_encode(")"), "%29");
    assert_eq!(url_encode("f(x)"), "f%28x%29");
}

// ── error tests (9 migrated) ────────────────────────────────────────────────

#[test]
fn api_error_display() {
    let err = AriError::Api {
        status: 404,
        message: "Not Found".to_owned(),
    };
    assert_eq!(err.to_string(), "API error 404: Not Found");
}

#[test]
fn websocket_error_display() {
    let err = AriError::WebSocket("connection reset".to_owned());
    let msg = err.to_string();
    assert!(msg.contains("connection reset"), "got: {msg}");
}

#[test]
fn disconnected_error_display() {
    let err = AriError::Disconnected;
    assert_eq!(err.to_string(), "client is disconnected");
}

#[test]
fn invalid_url_error_display() {
    let err = AriError::InvalidUrl("bad://url".to_owned());
    let msg = err.to_string();
    assert!(msg.contains("bad://url"), "got: {msg}");
}

#[test]
fn json_error_display() {
    let json_err = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
    let err = AriError::Json(json_err);
    let msg = err.to_string();
    assert!(msg.contains("JSON error"), "got: {msg}");
}

#[test]
fn io_error_display() {
    let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broke");
    let err = AriError::Io(io_err);
    let msg = err.to_string();
    assert!(msg.contains("pipe broke"), "got: {msg}");
}

#[test]
fn connection_error_display() {
    let err = AriError::Connection(ConnectionError::Closed);
    let msg = err.to_string();
    assert!(msg.contains("closed unexpectedly"), "got: {msg}");
}

#[test]
fn auth_error_display() {
    let err = AriError::Auth(AuthError::InvalidCredentials);
    let msg = err.to_string();
    assert!(msg.contains("invalid credentials"), "got: {msg}");
}

#[test]
fn errors_are_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<AriError>();
}

// ── event tests (14 migrated) ───────────────────────────────────────────────

#[test]
fn deserialize_stasis_start() {
    let json = r#"{
        "type": "StasisStart",
        "channel": {
            "id": "1234.5",
            "name": "PJSIP/alice-00000001",
            "state": "Ring",
            "caller": { "name": "Alice", "number": "1001" },
            "connected": { "name": "", "number": "" },
            "dialplan": { "context": "default", "exten": "100", "priority": 1 }
        },
        "args": ["arg1", "arg2"]
    }"#;

    let event: AriEvent = serde_json::from_str(json).expect("stasis start should deserialize");

    match event {
        AriEvent::StasisStart {
            channel,
            args,
            replace_channel,
        } => {
            assert_eq!(channel.id, "1234.5");
            assert_eq!(channel.name, "PJSIP/alice-00000001");
            assert_eq!(channel.state, "Ring");
            assert_eq!(channel.caller.name, "Alice");
            assert_eq!(channel.caller.number, "1001");
            assert_eq!(args, vec!["arg1", "arg2"]);
            assert!(replace_channel.is_none());
        }
        other => panic!("expected StasisStart, got {other:?}"),
    }
}

#[test]
fn deserialize_channel_dtmf_received() {
    let json = r#"{
        "type": "ChannelDtmfReceived",
        "channel": {
            "id": "chan-1",
            "name": "PJSIP/bob-00000002",
            "state": "Up"
        },
        "digit": "5",
        "duration_ms": 120
    }"#;

    let event: AriEvent = serde_json::from_str(json).expect("dtmf received should deserialize");

    match event {
        AriEvent::ChannelDtmfReceived {
            channel,
            digit,
            duration_ms,
        } => {
            assert_eq!(channel.id, "chan-1");
            assert_eq!(digit, "5");
            assert_eq!(duration_ms, 120);
        }
        other => panic!("expected ChannelDtmfReceived, got {other:?}"),
    }
}

#[test]
fn deserialize_unknown_event() {
    let json = r#"{
        "type": "SomeNewEventType",
        "data": "whatever"
    }"#;

    let event: AriEvent =
        serde_json::from_str(json).expect("unknown event types should not fail deserialization");

    assert!(matches!(event, AriEvent::Unknown));
}

#[test]
fn deserialize_stasis_start_minimal() {
    // no optional fields provided
    let json = r#"{
        "type": "StasisStart",
        "channel": {
            "id": "abc",
            "name": "SIP/trunk-00000001",
            "state": "Ring"
        }
    }"#;

    let event: AriEvent =
        serde_json::from_str(json).expect("minimal stasis start should deserialize");

    match event {
        AriEvent::StasisStart {
            channel,
            args,
            replace_channel,
        } => {
            assert_eq!(channel.id, "abc");
            assert!(args.is_empty());
            assert!(replace_channel.is_none());
        }
        other => panic!("expected StasisStart, got {other:?}"),
    }
}

#[test]
fn deserialize_dial_event() {
    let json = r#"{
        "type": "Dial",
        "peer": {
            "id": "peer-1",
            "name": "PJSIP/200-00000002",
            "state": "Ring"
        },
        "caller": {
            "id": "caller-1",
            "name": "PJSIP/100-00000001",
            "state": "Up"
        },
        "dialstatus": "RINGING",
        "dialstring": "200"
    }"#;
    let event: AriEvent = serde_json::from_str(json).expect("dial should deserialize");
    match event {
        AriEvent::Dial {
            peer,
            caller,
            dialstatus,
            ..
        } => {
            assert_eq!(peer.id, "peer-1");
            assert!(caller.is_some());
            assert_eq!(dialstatus, "RINGING");
        }
        other => panic!("expected Dial, got {other:?}"),
    }
}

#[test]
fn deserialize_channel_hold_event() {
    let json = r#"{
        "type": "ChannelHold",
        "channel": {
            "id": "hold-1",
            "name": "PJSIP/100-00000001",
            "state": "Up"
        },
        "musicclass": "default"
    }"#;
    let event: AriEvent = serde_json::from_str(json).expect("hold should deserialize");
    match event {
        AriEvent::ChannelHold {
            channel,
            musicclass,
        } => {
            assert_eq!(channel.id, "hold-1");
            assert_eq!(musicclass.as_deref(), Some("default"));
        }
        other => panic!("expected ChannelHold, got {other:?}"),
    }
}

#[test]
fn deserialize_bridge_blind_transfer() {
    let json = r#"{
        "type": "BridgeBlindTransfer",
        "channel": {
            "id": "xfer-1",
            "name": "PJSIP/100-00000001",
            "state": "Up"
        },
        "exten": "200",
        "context": "default",
        "result": "Success",
        "is_external": false
    }"#;
    let event: AriEvent = serde_json::from_str(json).expect("blind transfer should deserialize");
    match event {
        AriEvent::BridgeBlindTransfer {
            channel,
            exten,
            context,
            result,
            is_external,
            ..
        } => {
            assert_eq!(channel.id, "xfer-1");
            assert_eq!(exten, "200");
            assert_eq!(context, "default");
            assert_eq!(result, "Success");
            assert!(!is_external);
        }
        other => panic!("expected BridgeBlindTransfer, got {other:?}"),
    }
}

#[test]
fn deserialize_peer_status_change() {
    let json = r#"{
        "type": "PeerStatusChange",
        "endpoint": {
            "technology": "PJSIP",
            "resource": "100",
            "state": "online"
        },
        "peer": {
            "peer_status": "Reachable",
            "address": "192.168.1.100",
            "port": "5060"
        }
    }"#;
    let event: AriEvent = serde_json::from_str(json).expect("peer status should deserialize");
    match event {
        AriEvent::PeerStatusChange { endpoint, peer } => {
            assert_eq!(endpoint.resource, "100");
            assert_eq!(peer.peer_status, "Reachable");
        }
        other => panic!("expected PeerStatusChange, got {other:?}"),
    }
}

#[test]
fn deserialize_contact_status_change() {
    let json = r#"{
        "type": "ContactStatusChange",
        "contact_info": {
            "uri": "sip:100@192.168.1.100:5060",
            "contact_status": "Reachable",
            "aor": "100"
        },
        "endpoint": {
            "technology": "PJSIP",
            "resource": "100"
        }
    }"#;
    let event: AriEvent = serde_json::from_str(json).expect("contact status should deserialize");
    match event {
        AriEvent::ContactStatusChange {
            contact_info,
            endpoint,
        } => {
            assert_eq!(contact_info.aor, "100");
            assert_eq!(endpoint.resource, "100");
        }
        other => panic!("expected ContactStatusChange, got {other:?}"),
    }
}

#[test]
fn deserialize_device_state_changed() {
    let json = r#"{
        "type": "DeviceStateChanged",
        "device_state": {
            "name": "PJSIP/100",
            "state": "INUSE"
        }
    }"#;
    let event: AriEvent = serde_json::from_str(json).expect("device state should deserialize");
    match event {
        AriEvent::DeviceStateChanged { device_state } => {
            assert_eq!(device_state.name, "PJSIP/100");
            assert_eq!(device_state.state, "INUSE");
        }
        other => panic!("expected DeviceStateChanged, got {other:?}"),
    }
}

#[test]
fn deserialize_playback_continuing() {
    let json = r#"{
        "type": "PlaybackContinuing",
        "playback": {
            "id": "pb-1",
            "media_uri": "sound:hello-world",
            "state": "playing",
            "target_uri": "channel:abc"
        }
    }"#;
    let event: AriEvent =
        serde_json::from_str(json).expect("playback continuing should deserialize");
    assert!(matches!(event, AriEvent::PlaybackContinuing { .. }));
}

#[test]
fn deserialize_recording_failed() {
    let json = r#"{
        "type": "RecordingFailed",
        "recording": {
            "name": "rec-1",
            "format": "wav",
            "state": "failed",
            "target_uri": "channel:abc"
        }
    }"#;
    let event: AriEvent = serde_json::from_str(json).expect("recording failed should deserialize");
    assert!(matches!(event, AriEvent::RecordingFailed { .. }));
}

#[test]
fn deserialize_ari_message_with_metadata() {
    let json = r#"{
        "type": "StasisStart",
        "application": "my-app",
        "timestamp": "2024-01-15T10:30:00.000+0000",
        "asterisk_id": "00:11:22:33:44:55",
        "channel": {
            "id": "1234.5",
            "name": "PJSIP/alice-00000001",
            "state": "Ring"
        }
    }"#;
    let msg: AriMessage = serde_json::from_str(json).expect("should deserialize");
    assert_eq!(msg.application, "my-app");
    assert_eq!(msg.timestamp, "2024-01-15T10:30:00.000+0000");
    assert_eq!(msg.asterisk_id.as_deref(), Some("00:11:22:33:44:55"));
    assert!(matches!(msg.event, AriEvent::StasisStart { .. }));
}

#[test]
fn deserialize_ari_message_without_metadata() {
    let json = r#"{
        "type": "StasisEnd",
        "channel": {
            "id": "1234.5",
            "name": "PJSIP/alice-00000001",
            "state": "Up"
        }
    }"#;
    let msg: AriMessage = serde_json::from_str(json).expect("should deserialize");
    assert_eq!(msg.application, "");
    assert_eq!(msg.timestamp, "");
    assert!(msg.asterisk_id.is_none());
    assert!(matches!(msg.event, AriEvent::StasisEnd { .. }));
}

// ── new event variant tests ─────────────────────────────────────────────────

fn minimal_channel_json() -> &'static str {
    r#"{"id": "ch1", "name": "PJSIP/100", "state": "Up"}"#
}

fn minimal_bridge_json() -> &'static str {
    r#"{"id": "br1", "technology": "simple_bridge", "bridge_type": "mixing"}"#
}

fn minimal_playback_json() -> &'static str {
    r#"{"id": "pb1", "media_uri": "sound:hello", "state": "playing"}"#
}

fn minimal_recording_json() -> &'static str {
    r#"{"name": "rec1", "format": "wav", "state": "recording"}"#
}

fn minimal_endpoint_json() -> &'static str {
    r#"{"technology": "PJSIP", "resource": "100"}"#
}

#[test]
fn deserialize_stasis_end() {
    let json = format!(
        r#"{{"type": "StasisEnd", "channel": {}}}"#,
        minimal_channel_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::StasisEnd { channel } => assert_eq!(channel.id, "ch1"),
        other => panic!("expected StasisEnd, got {other:?}"),
    }
}

#[test]
fn deserialize_channel_created() {
    let json = format!(
        r#"{{"type": "ChannelCreated", "channel": {}}}"#,
        minimal_channel_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::ChannelCreated { channel } => assert_eq!(channel.id, "ch1"),
        other => panic!("expected ChannelCreated, got {other:?}"),
    }
}

#[test]
fn deserialize_channel_destroyed() {
    let json = format!(
        r#"{{"type": "ChannelDestroyed", "channel": {}, "cause": 16, "cause_txt": "Normal Clearing"}}"#,
        minimal_channel_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::ChannelDestroyed {
            channel,
            cause,
            cause_txt,
        } => {
            assert_eq!(channel.id, "ch1");
            assert_eq!(cause, 16);
            assert_eq!(cause_txt, "Normal Clearing");
        }
        other => panic!("expected ChannelDestroyed, got {other:?}"),
    }
}

#[test]
fn deserialize_channel_state_change() {
    let json = format!(
        r#"{{"type": "ChannelStateChange", "channel": {}}}"#,
        minimal_channel_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::ChannelStateChange { channel } => assert_eq!(channel.state, "Up"),
        other => panic!("expected ChannelStateChange, got {other:?}"),
    }
}

#[test]
fn deserialize_channel_hangup_request() {
    let json = format!(
        r#"{{"type": "ChannelHangupRequest", "channel": {}}}"#,
        minimal_channel_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    assert!(matches!(event, AriEvent::ChannelHangupRequest { .. }));
}

#[test]
fn deserialize_channel_varset() {
    let json = format!(
        r#"{{"type": "ChannelVarset", "channel": {}, "variable": "CDR(src)", "value": "1001"}}"#,
        minimal_channel_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::ChannelVarset {
            channel,
            variable,
            value,
        } => {
            assert!(channel.is_some());
            assert_eq!(variable, "CDR(src)");
            assert_eq!(value, "1001");
        }
        other => panic!("expected ChannelVarset, got {other:?}"),
    }
}

#[test]
fn deserialize_channel_varset_no_channel() {
    let json = r#"{"type": "ChannelVarset", "variable": "GLOBAL_VAR", "value": "yes"}"#;
    let event: AriEvent = serde_json::from_str(json).expect("deser");
    match event {
        AriEvent::ChannelVarset { channel, .. } => {
            assert!(channel.is_none());
        }
        other => panic!("expected ChannelVarset, got {other:?}"),
    }
}

#[test]
fn deserialize_bridge_created() {
    let json = format!(
        r#"{{"type": "BridgeCreated", "bridge": {}}}"#,
        minimal_bridge_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::BridgeCreated { bridge } => assert_eq!(bridge.id, "br1"),
        other => panic!("expected BridgeCreated, got {other:?}"),
    }
}

#[test]
fn deserialize_bridge_destroyed() {
    let json = format!(
        r#"{{"type": "BridgeDestroyed", "bridge": {}}}"#,
        minimal_bridge_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::BridgeDestroyed { bridge } => assert_eq!(bridge.id, "br1"),
        other => panic!("expected BridgeDestroyed, got {other:?}"),
    }
}

#[test]
fn deserialize_channel_entered_bridge() {
    let json = format!(
        r#"{{"type": "ChannelEnteredBridge", "bridge": {}, "channel": {}}}"#,
        minimal_bridge_json(),
        minimal_channel_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::ChannelEnteredBridge { bridge, channel } => {
            assert_eq!(bridge.id, "br1");
            assert_eq!(channel.id, "ch1");
        }
        other => panic!("expected ChannelEnteredBridge, got {other:?}"),
    }
}

#[test]
fn deserialize_channel_left_bridge() {
    let json = format!(
        r#"{{"type": "ChannelLeftBridge", "bridge": {}, "channel": {}}}"#,
        minimal_bridge_json(),
        minimal_channel_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::ChannelLeftBridge { bridge, channel } => {
            assert_eq!(bridge.id, "br1");
            assert_eq!(channel.id, "ch1");
        }
        other => panic!("expected ChannelLeftBridge, got {other:?}"),
    }
}

#[test]
fn deserialize_playback_started() {
    let json = format!(
        r#"{{"type": "PlaybackStarted", "playback": {}}}"#,
        minimal_playback_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::PlaybackStarted { playback } => assert_eq!(playback.id, "pb1"),
        other => panic!("expected PlaybackStarted, got {other:?}"),
    }
}

#[test]
fn deserialize_playback_finished() {
    let json = format!(
        r#"{{"type": "PlaybackFinished", "playback": {}}}"#,
        minimal_playback_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::PlaybackFinished { playback } => assert_eq!(playback.state, "playing"),
        other => panic!("expected PlaybackFinished, got {other:?}"),
    }
}

#[test]
fn deserialize_recording_started() {
    let json = format!(
        r#"{{"type": "RecordingStarted", "recording": {}}}"#,
        minimal_recording_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::RecordingStarted { recording } => assert_eq!(recording.name, "rec1"),
        other => panic!("expected RecordingStarted, got {other:?}"),
    }
}

#[test]
fn deserialize_recording_finished() {
    let json = format!(
        r#"{{"type": "RecordingFinished", "recording": {}}}"#,
        minimal_recording_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::RecordingFinished { recording } => assert_eq!(recording.format, "wav"),
        other => panic!("expected RecordingFinished, got {other:?}"),
    }
}

#[test]
fn deserialize_channel_caller_id() {
    let json = format!(
        r#"{{"type": "ChannelCallerId", "channel": {}, "caller_presentation": 0, "caller_presentation_txt": "Allowed"}}"#,
        minimal_channel_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::ChannelCallerId {
            channel,
            caller_presentation,
            caller_presentation_txt,
        } => {
            assert_eq!(channel.id, "ch1");
            assert_eq!(caller_presentation, 0);
            assert_eq!(caller_presentation_txt, "Allowed");
        }
        other => panic!("expected ChannelCallerId, got {other:?}"),
    }
}

#[test]
fn deserialize_channel_connected_line() {
    let json = format!(
        r#"{{"type": "ChannelConnectedLine", "channel": {}}}"#,
        minimal_channel_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    assert!(matches!(event, AriEvent::ChannelConnectedLine { .. }));
}

#[test]
fn deserialize_channel_dialplan() {
    let json = format!(
        r#"{{"type": "ChannelDialplan", "channel": {}, "dialplan_app": "Stasis", "dialplan_app_data": "myapp"}}"#,
        minimal_channel_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::ChannelDialplan {
            dialplan_app,
            dialplan_app_data,
            ..
        } => {
            assert_eq!(dialplan_app, "Stasis");
            assert_eq!(dialplan_app_data, "myapp");
        }
        other => panic!("expected ChannelDialplan, got {other:?}"),
    }
}

#[test]
fn deserialize_channel_hold_no_musicclass() {
    let json = format!(
        r#"{{"type": "ChannelHold", "channel": {}}}"#,
        minimal_channel_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::ChannelHold { musicclass, .. } => {
            assert!(musicclass.is_none());
        }
        other => panic!("expected ChannelHold, got {other:?}"),
    }
}

#[test]
fn deserialize_channel_unhold() {
    let json = format!(
        r#"{{"type": "ChannelUnhold", "channel": {}}}"#,
        minimal_channel_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    assert!(matches!(event, AriEvent::ChannelUnhold { .. }));
}

#[test]
fn deserialize_channel_talking_started() {
    let json = format!(
        r#"{{"type": "ChannelTalkingStarted", "channel": {}}}"#,
        minimal_channel_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    assert!(matches!(event, AriEvent::ChannelTalkingStarted { .. }));
}

#[test]
fn deserialize_channel_talking_finished() {
    let json = format!(
        r#"{{"type": "ChannelTalkingFinished", "channel": {}, "duration": 5}}"#,
        minimal_channel_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::ChannelTalkingFinished { duration, .. } => {
            assert_eq!(duration, 5);
        }
        other => panic!("expected ChannelTalkingFinished, got {other:?}"),
    }
}

#[test]
fn deserialize_channel_tone_detected() {
    let json = format!(
        r#"{{"type": "ChannelToneDetected", "channel": {}}}"#,
        minimal_channel_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    assert!(matches!(event, AriEvent::ChannelToneDetected { .. }));
}

#[test]
fn deserialize_channel_transfer() {
    let json = format!(
        r#"{{"type": "ChannelTransfer", "channel": {}, "state": "Active"}}"#,
        minimal_channel_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::ChannelTransfer { channel, state, .. } => {
            assert_eq!(channel.id, "ch1");
            assert_eq!(state.as_deref(), Some("Active"));
        }
        other => panic!("expected ChannelTransfer, got {other:?}"),
    }
}

#[test]
fn deserialize_channel_userevent() {
    let json = format!(
        r#"{{"type": "ChannelUserevent", "channel": {}, "eventname": "MyCustomEvent", "userevent": {{"key": "val"}}}}"#,
        minimal_channel_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::ChannelUserevent {
            channel,
            eventname,
            userevent,
            ..
        } => {
            assert!(channel.is_some());
            assert_eq!(eventname, "MyCustomEvent");
            assert_eq!(userevent["key"], "val");
        }
        other => panic!("expected ChannelUserevent, got {other:?}"),
    }
}

#[test]
fn deserialize_channel_userevent_minimal() {
    let json = r#"{"type": "ChannelUserevent", "eventname": "Bare", "userevent": {}}"#;
    let event: AriEvent = serde_json::from_str(json).expect("deser");
    match event {
        AriEvent::ChannelUserevent {
            channel,
            bridge,
            endpoint,
            ..
        } => {
            assert!(channel.is_none());
            assert!(bridge.is_none());
            assert!(endpoint.is_none());
        }
        other => panic!("expected ChannelUserevent, got {other:?}"),
    }
}

#[test]
fn deserialize_bridge_attended_transfer() {
    let json = r#"{
        "type": "BridgeAttendedTransfer",
        "transferer_first_leg": {"id": "leg1", "name": "PJSIP/100", "state": "Up"},
        "transferer_second_leg": {"id": "leg2", "name": "PJSIP/200", "state": "Up"},
        "result": "Success",
        "destination_type": "bridge",
        "is_external": false
    }"#;
    let event: AriEvent = serde_json::from_str(json).expect("deser");
    match event {
        AriEvent::BridgeAttendedTransfer {
            transferer_first_leg,
            transferer_second_leg,
            result,
            destination_type,
            is_external,
            ..
        } => {
            assert_eq!(transferer_first_leg.id, "leg1");
            assert_eq!(transferer_second_leg.id, "leg2");
            assert_eq!(result, "Success");
            assert_eq!(destination_type, "bridge");
            assert!(!is_external);
        }
        other => panic!("expected BridgeAttendedTransfer, got {other:?}"),
    }
}

#[test]
fn deserialize_bridge_merged() {
    let json = r#"{
        "type": "BridgeMerged",
        "bridge": {"id": "br1", "technology": "simple_bridge", "bridge_type": "mixing"},
        "bridge_from": {"id": "br2", "technology": "simple_bridge", "bridge_type": "holding"}
    }"#;
    let event: AriEvent = serde_json::from_str(json).expect("deser");
    match event {
        AriEvent::BridgeMerged {
            bridge,
            bridge_from,
        } => {
            assert_eq!(bridge.id, "br1");
            assert_eq!(bridge_from.id, "br2");
        }
        other => panic!("expected BridgeMerged, got {other:?}"),
    }
}

#[test]
fn deserialize_bridge_video_source_changed() {
    let json = format!(
        r#"{{"type": "BridgeVideoSourceChanged", "bridge": {}, "old_video_source_id": "ch-old"}}"#,
        minimal_bridge_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::BridgeVideoSourceChanged {
            bridge,
            old_video_source_id,
        } => {
            assert_eq!(bridge.id, "br1");
            assert_eq!(old_video_source_id.as_deref(), Some("ch-old"));
        }
        other => panic!("expected BridgeVideoSourceChanged, got {other:?}"),
    }
}

#[test]
fn deserialize_endpoint_state_change() {
    let json = format!(
        r#"{{"type": "EndpointStateChange", "endpoint": {}}}"#,
        minimal_endpoint_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::EndpointStateChange { endpoint } => {
            assert_eq!(endpoint.technology, "PJSIP");
        }
        other => panic!("expected EndpointStateChange, got {other:?}"),
    }
}

#[test]
fn deserialize_application_move_failed() {
    let json = format!(
        r#"{{"type": "ApplicationMoveFailed", "channel": {}, "destination": "other-app", "args": ["a1"]}}"#,
        minimal_channel_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::ApplicationMoveFailed {
            channel,
            destination,
            args,
        } => {
            assert_eq!(channel.id, "ch1");
            assert_eq!(destination, "other-app");
            assert_eq!(args, vec!["a1"]);
        }
        other => panic!("expected ApplicationMoveFailed, got {other:?}"),
    }
}

#[test]
fn deserialize_application_registered() {
    let json = r#"{"type": "ApplicationRegistered"}"#;
    let event: AriEvent = serde_json::from_str(json).expect("deser");
    assert!(matches!(event, AriEvent::ApplicationRegistered {}));
}

#[test]
fn deserialize_application_replaced() {
    let json = r#"{"type": "ApplicationReplaced"}"#;
    let event: AriEvent = serde_json::from_str(json).expect("deser");
    assert!(matches!(event, AriEvent::ApplicationReplaced {}));
}

#[test]
fn deserialize_application_unregistered() {
    let json = r#"{"type": "ApplicationUnregistered"}"#;
    let event: AriEvent = serde_json::from_str(json).expect("deser");
    assert!(matches!(event, AriEvent::ApplicationUnregistered {}));
}

#[test]
fn deserialize_text_message_received() {
    let json = format!(
        r#"{{
            "type": "TextMessageReceived",
            "message": {{"from": "sip:100@pbx", "to": "sip:200@pbx", "body": "hello"}},
            "endpoint": {}
        }}"#,
        minimal_endpoint_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::TextMessageReceived { message, endpoint } => {
            assert_eq!(message.from, "sip:100@pbx");
            assert_eq!(message.to, "sip:200@pbx");
            assert_eq!(message.body, "hello");
            assert!(endpoint.is_some());
        }
        other => panic!("expected TextMessageReceived, got {other:?}"),
    }
}

#[test]
fn deserialize_text_message_received_no_endpoint() {
    let json = r#"{
        "type": "TextMessageReceived",
        "message": {"from": "a", "to": "b", "body": "c"}
    }"#;
    let event: AriEvent = serde_json::from_str(json).expect("deser");
    match event {
        AriEvent::TextMessageReceived { endpoint, .. } => {
            assert!(endpoint.is_none());
        }
        other => panic!("expected TextMessageReceived, got {other:?}"),
    }
}

#[test]
fn deserialize_rest_response() {
    let json = r#"{
        "type": "RESTResponse",
        "status_code": 200,
        "reason_phrase": "OK",
        "uri": "/ari/channels",
        "request_id": "req-1",
        "transaction_id": "tx-1",
        "content_type": "application/json",
        "message_body": "{}"
    }"#;
    let event: AriEvent = serde_json::from_str(json).expect("deser");
    match event {
        AriEvent::RESTResponse {
            status_code,
            reason_phrase,
            uri,
            request_id,
            transaction_id,
            content_type,
            message_body,
        } => {
            assert_eq!(status_code, 200);
            assert_eq!(reason_phrase, "OK");
            assert_eq!(uri, "/ari/channels");
            assert_eq!(request_id, "req-1");
            assert_eq!(transaction_id, "tx-1");
            assert_eq!(content_type.as_deref(), Some("application/json"));
            assert_eq!(message_body.as_deref(), Some("{}"));
        }
        other => panic!("expected RESTResponse, got {other:?}"),
    }
}

#[test]
fn deserialize_rest_response_minimal() {
    let json = r#"{
        "type": "RESTResponse",
        "status_code": 204,
        "reason_phrase": "No Content",
        "uri": "/ari/bridges",
        "request_id": "r2",
        "transaction_id": "t2"
    }"#;
    let event: AriEvent = serde_json::from_str(json).expect("deser");
    match event {
        AriEvent::RESTResponse {
            content_type,
            message_body,
            ..
        } => {
            assert!(content_type.is_none());
            assert!(message_body.is_none());
        }
        other => panic!("expected RESTResponse, got {other:?}"),
    }
}

#[test]
fn deserialize_dial_minimal() {
    let json = r#"{
        "type": "Dial",
        "peer": {"id": "p1", "name": "PJSIP/300", "state": "Ring"},
        "dialstatus": "ANSWER"
    }"#;
    let event: AriEvent = serde_json::from_str(json).expect("deser");
    match event {
        AriEvent::Dial {
            caller,
            forwarded,
            dialstring,
            forward,
            ..
        } => {
            assert!(caller.is_none());
            assert!(forwarded.is_none());
            assert!(dialstring.is_none());
            assert!(forward.is_none());
        }
        other => panic!("expected Dial, got {other:?}"),
    }
}

// ── event inner type tests ──────────────────────────────────────────────────

#[test]
fn deserialize_channel_with_all_fields() {
    let json = r#"{
        "id": "chan-full",
        "name": "PJSIP/alice-00000005",
        "state": "Up",
        "caller": {"name": "Alice", "number": "1001"},
        "connected": {"name": "Bob", "number": "2002"},
        "dialplan": {"context": "internal", "exten": "300", "priority": 2}
    }"#;
    let ch: Channel = serde_json::from_str(json).expect("deser");
    assert_eq!(ch.id, "chan-full");
    assert_eq!(ch.caller.name, "Alice");
    assert_eq!(ch.connected.number, "2002");
    assert_eq!(ch.dialplan.context, "internal");
    assert_eq!(ch.dialplan.priority, 2);
}

#[test]
fn deserialize_channel_minimal() {
    let json = r#"{"id": "x", "name": "SIP/t", "state": "Ring"}"#;
    let ch: Channel = serde_json::from_str(json).expect("deser");
    assert_eq!(ch.caller.name, "");
    assert_eq!(ch.dialplan.context, "");
    assert_eq!(ch.dialplan.priority, 0);
}

#[test]
fn deserialize_caller_id_default() {
    let cid: CallerId = serde_json::from_str(r#"{}"#).expect("deser");
    assert_eq!(cid.name, "");
    assert_eq!(cid.number, "");
}

#[test]
fn deserialize_dialplan_cep_default() {
    let cep: DialplanCep = serde_json::from_str(r#"{}"#).expect("deser");
    assert_eq!(cep.context, "");
    assert_eq!(cep.exten, "");
    assert_eq!(cep.priority, 0);
}

#[test]
fn deserialize_bridge_with_channels() {
    let json = r#"{
        "id": "br-full",
        "technology": "softmix",
        "bridge_type": "mixing",
        "channels": ["ch1", "ch2"]
    }"#;
    let br: Bridge = serde_json::from_str(json).expect("deser");
    assert_eq!(br.id, "br-full");
    assert_eq!(br.channels, vec!["ch1", "ch2"]);
}

#[test]
fn deserialize_bridge_no_channels() {
    let json = r#"{"id": "br-empty", "technology": "simple_bridge", "bridge_type": "holding"}"#;
    let br: Bridge = serde_json::from_str(json).expect("deser");
    assert!(br.channels.is_empty());
}

#[test]
fn deserialize_playback_type() {
    let json = r#"{
        "id": "pb-full",
        "media_uri": "sound:tt-monkeys",
        "state": "done",
        "target_uri": "channel:abc"
    }"#;
    let pb: Playback = serde_json::from_str(json).expect("deser");
    assert_eq!(pb.id, "pb-full");
    assert_eq!(pb.media_uri, "sound:tt-monkeys");
    assert_eq!(pb.state, "done");
    assert_eq!(pb.target_uri, "channel:abc");
}

#[test]
fn deserialize_live_recording_type() {
    let json = r#"{
        "name": "rec-full",
        "format": "wav",
        "state": "recording",
        "target_uri": "channel:xyz"
    }"#;
    let rec: LiveRecording = serde_json::from_str(json).expect("deser");
    assert_eq!(rec.name, "rec-full");
    assert_eq!(rec.format, "wav");
    assert_eq!(rec.target_uri, "channel:xyz");
}

#[test]
fn deserialize_contact_info() {
    let json = r#"{
        "uri": "sip:100@10.0.0.1:5060",
        "contact_status": "Reachable",
        "aor": "100",
        "roundtrip_usec": "1234"
    }"#;
    let ci: ContactInfo = serde_json::from_str(json).expect("deser");
    assert_eq!(ci.uri, "sip:100@10.0.0.1:5060");
    assert_eq!(ci.roundtrip_usec.as_deref(), Some("1234"));
}

#[test]
fn deserialize_peer_type() {
    let json = r#"{
        "peer_status": "Reachable",
        "address": "10.0.0.1",
        "port": "5060",
        "cause": "200",
        "time": "12345"
    }"#;
    let p: Peer = serde_json::from_str(json).expect("deser");
    assert_eq!(p.peer_status, "Reachable");
    assert_eq!(p.address.as_deref(), Some("10.0.0.1"));
    assert_eq!(p.cause.as_deref(), Some("200"));
    assert_eq!(p.time.as_deref(), Some("12345"));
}

#[test]
fn deserialize_event_endpoint() {
    let json = r#"{
        "technology": "PJSIP",
        "resource": "200",
        "state": "online",
        "channel_ids": ["ch1", "ch2"]
    }"#;
    let ep: Endpoint = serde_json::from_str(json).expect("deser");
    assert_eq!(ep.technology, "PJSIP");
    assert_eq!(ep.resource, "200");
    assert_eq!(ep.state.as_deref(), Some("online"));
    assert_eq!(ep.channel_ids.len(), 2);
}

#[test]
fn deserialize_event_device_state() {
    let json = r#"{"name": "PJSIP/300", "state": "NOT_INUSE"}"#;
    let ds: DeviceState = serde_json::from_str(json).expect("deser");
    assert_eq!(ds.name, "PJSIP/300");
    assert_eq!(ds.state, "NOT_INUSE");
}

#[test]
fn deserialize_text_message() {
    let json = r#"{"from": "a@b", "to": "c@d", "body": "hi"}"#;
    let tm: TextMessage = serde_json::from_str(json).expect("deser");
    assert_eq!(tm.from, "a@b");
    assert_eq!(tm.to, "c@d");
    assert_eq!(tm.body, "hi");
}

#[test]
fn deserialize_refer_to() {
    let json = r#"{
        "destination_channel": {"id": "d1", "name": "PJSIP/100", "state": "Ring"},
        "bridge": {"id": "br1", "technology": "simple_bridge", "bridge_type": "mixing"}
    }"#;
    let rt: ReferTo = serde_json::from_str(json).expect("deser");
    assert!(rt.destination_channel.is_some());
    assert!(rt.bridge.is_some());
    assert!(rt.connected_channel.is_none());
}

#[test]
fn deserialize_referred_by() {
    let json = r#"{
        "source_channel": {"id": "s1", "name": "PJSIP/200", "state": "Up"}
    }"#;
    let rb: ReferredBy = serde_json::from_str(json).expect("deser");
    assert_eq!(rb.source_channel.id, "s1");
    assert!(rb.connected_channel.is_none());
    assert!(rb.bridge.is_none());
}

// ── resource data type tests ────────────────────────────────────────────────

#[test]
fn deserialize_resource_endpoint() {
    let json = r#"{
        "technology": "PJSIP",
        "resource": "400",
        "state": "online",
        "channel_ids": ["c1"]
    }"#;
    let ep: ResourceEndpoint = serde_json::from_str(json).expect("deser");
    assert_eq!(ep.technology, "PJSIP");
    assert_eq!(ep.resource, "400");
    assert_eq!(ep.state.as_deref(), Some("online"));
    assert_eq!(ep.channel_ids, vec!["c1"]);
}

#[test]
fn deserialize_resource_endpoint_minimal() {
    let json = r#"{"technology": "SIP", "resource": "trunk"}"#;
    let ep: ResourceEndpoint = serde_json::from_str(json).expect("deser");
    assert!(ep.state.is_none());
    assert!(ep.channel_ids.is_empty());
}

#[test]
fn deserialize_resource_device_state() {
    let json = r#"{"name": "Custom:mydev", "state": "BUSY"}"#;
    let ds: ResourceDeviceState = serde_json::from_str(json).expect("deser");
    assert_eq!(ds.name, "Custom:mydev");
    assert_eq!(ds.state, "BUSY");
}

#[test]
fn deserialize_stored_recording() {
    let json = r#"{"name": "my-recording", "format": "wav"}"#;
    let sr: StoredRecording = serde_json::from_str(json).expect("deser");
    assert_eq!(sr.name, "my-recording");
    assert_eq!(sr.format, "wav");
}

#[test]
fn deserialize_sound() {
    let json = r#"{
        "id": "hello-world",
        "text": "Hello World",
        "formats": [
            {"language": "en", "format": "gsm"},
            {"language": "en", "format": "wav"}
        ]
    }"#;
    let s: Sound = serde_json::from_str(json).expect("deser");
    assert_eq!(s.id, "hello-world");
    assert_eq!(s.text, "Hello World");
    assert_eq!(s.formats.len(), 2);
    assert_eq!(s.formats[0].language, "en");
    assert_eq!(s.formats[1].format, "wav");
}

#[test]
fn deserialize_sound_minimal() {
    let json = r#"{"id": "beep"}"#;
    let s: Sound = serde_json::from_str(json).expect("deser");
    assert_eq!(s.id, "beep");
    assert_eq!(s.text, "");
    assert!(s.formats.is_empty());
}

#[test]
fn deserialize_sound_format() {
    let json = r#"{"language": "es", "format": "sln16"}"#;
    let sf: SoundFormat = serde_json::from_str(json).expect("deser");
    assert_eq!(sf.language, "es");
    assert_eq!(sf.format, "sln16");
}

#[test]
fn deserialize_mailbox() {
    let json = r#"{"name": "1001@default", "old_messages": 3, "new_messages": 7}"#;
    let mb: Mailbox = serde_json::from_str(json).expect("deser");
    assert_eq!(mb.name, "1001@default");
    assert_eq!(mb.old_messages, 3);
    assert_eq!(mb.new_messages, 7);
}

#[test]
fn deserialize_application() {
    let json = r#"{
        "name": "my-stasis-app",
        "channel_ids": ["ch1", "ch2"],
        "bridge_ids": ["br1"],
        "endpoint_ids": [],
        "device_names": ["PJSIP/100"]
    }"#;
    let app: Application = serde_json::from_str(json).expect("deser");
    assert_eq!(app.name, "my-stasis-app");
    assert_eq!(app.channel_ids.len(), 2);
    assert_eq!(app.bridge_ids, vec!["br1"]);
    assert!(app.endpoint_ids.is_empty());
    assert_eq!(app.device_names, vec!["PJSIP/100"]);
}

#[test]
fn deserialize_application_minimal() {
    let json = r#"{"name": "bare-app"}"#;
    let app: Application = serde_json::from_str(json).expect("deser");
    assert_eq!(app.name, "bare-app");
    assert!(app.channel_ids.is_empty());
    assert!(app.bridge_ids.is_empty());
    assert!(app.endpoint_ids.is_empty());
    assert!(app.device_names.is_empty());
}

#[test]
fn deserialize_asterisk_info() {
    let json = r#"{
        "build": {"os": "Linux", "kernel": "5.10"},
        "config": {"name": "asterisk", "max_open_files": 1024},
        "status": {"startup_time": "2024-01-01"},
        "system": {"entity_id": "abcd", "version": "20.5.0"}
    }"#;
    let info: AsteriskInfo = serde_json::from_str(json).expect("deser");
    assert!(info.build.is_some());
    assert!(info.config.is_some());
    assert!(info.status.is_some());
    assert!(info.system.is_some());
}

#[test]
fn deserialize_asterisk_info_minimal() {
    let json = r#"{}"#;
    let info: AsteriskInfo = serde_json::from_str(json).expect("deser");
    assert!(info.build.is_none());
    assert!(info.config.is_none());
    assert!(info.status.is_none());
    assert!(info.system.is_none());
}

#[test]
fn deserialize_asterisk_ping() {
    let json = r#"{
        "asterisk_id": "00:11:22:33:44:55",
        "ping": "pong",
        "timestamp": "2024-01-15T12:00:00.000+0000"
    }"#;
    let ping: AsteriskPing = serde_json::from_str(json).expect("deser");
    assert_eq!(ping.asterisk_id, "00:11:22:33:44:55");
    assert_eq!(ping.ping, "pong");
    assert!(ping.timestamp.contains("2024"));
}

#[test]
fn deserialize_module() {
    let json = r#"{
        "name": "res_pjsip.so",
        "description": "Basic SIP resource",
        "use_count": 5,
        "status": "Running",
        "support_level": "core"
    }"#;
    let m: Module = serde_json::from_str(json).expect("deser");
    assert_eq!(m.name, "res_pjsip.so");
    assert_eq!(m.description, "Basic SIP resource");
    assert_eq!(m.use_count, 5);
    assert_eq!(m.status, "Running");
    assert_eq!(m.support_level.as_deref(), Some("core"));
}

#[test]
fn deserialize_module_no_support_level() {
    let json = r#"{
        "name": "app_custom.so",
        "description": "Custom app",
        "use_count": 0,
        "status": "Running"
    }"#;
    let m: Module = serde_json::from_str(json).expect("deser");
    assert!(m.support_level.is_none());
}

#[test]
fn deserialize_log_channel() {
    let json = r#"{
        "channel": "console",
        "type": "VERBOSE",
        "status": "Enabled",
        "configuration": "notice,warning,error"
    }"#;
    let lc: LogChannel = serde_json::from_str(json).expect("deser");
    assert_eq!(lc.channel, "console");
    assert_eq!(lc.log_type, "VERBOSE");
    assert_eq!(lc.status, "Enabled");
    assert_eq!(lc.configuration, "notice,warning,error");
}

#[test]
fn deserialize_config_tuple() {
    let json = r#"{"attribute": "max_contacts", "value": "5"}"#;
    let ct: ConfigTuple = serde_json::from_str(json).expect("deser");
    assert_eq!(ct.attribute, "max_contacts");
    assert_eq!(ct.value, "5");
}

#[test]
fn serialize_config_tuple() {
    let ct = ConfigTuple {
        attribute: "key".to_owned(),
        value: "val".to_owned(),
    };
    let json = serde_json::to_string(&ct).expect("ser");
    assert!(json.contains("\"attribute\":\"key\""));
    assert!(json.contains("\"value\":\"val\""));
}

#[test]
fn deserialize_asterisk_variable() {
    let json = r#"{"value": "some_value"}"#;
    let v: AsteriskVariable = serde_json::from_str(json).expect("deser");
    assert_eq!(v.value, "some_value");
}

#[test]
fn deserialize_channel_variable() {
    let json = r#"{"value": "channel_val"}"#;
    let v: ChannelVariable = serde_json::from_str(json).expect("deser");
    assert_eq!(v.value, "channel_val");
}

#[test]
fn serialize_originate_params_full() {
    let params = OriginateParams {
        endpoint: "PJSIP/100".to_owned(),
        extension: Some("100".to_owned()),
        context: Some("default".to_owned()),
        priority: Some(1),
        app: Some("myapp".to_owned()),
        app_args: Some("arg1,arg2".to_owned()),
        caller_id: Some("\"Test\" <1000>".to_owned()),
        timeout: Some(30),
    };
    let json = serde_json::to_string(&params).expect("ser");
    assert!(json.contains("\"endpoint\":\"PJSIP/100\""));
    assert!(json.contains("\"extension\":\"100\""));
    assert!(json.contains("\"context\":\"default\""));
    assert!(json.contains("\"priority\":1"));
    assert!(json.contains("\"app\":\"myapp\""));
    assert!(json.contains("\"timeout\":30"));
}

#[test]
fn serialize_originate_params_minimal() {
    let params = OriginateParams {
        endpoint: "PJSIP/200".to_owned(),
        ..Default::default()
    };
    let json = serde_json::to_string(&params).expect("ser");
    assert!(json.contains("\"endpoint\":\"PJSIP/200\""));
    // none fields should be skipped
    assert!(!json.contains("extension"));
    assert!(!json.contains("context"));
    assert!(!json.contains("priority"));
    assert!(!json.contains("app_args"));
    assert!(!json.contains("caller_id"));
    assert!(!json.contains("timeout"));
}

#[test]
fn originate_params_default() {
    let params = OriginateParams::default();
    assert_eq!(params.endpoint, "");
    assert!(params.extension.is_none());
    assert!(params.context.is_none());
    assert!(params.priority.is_none());
    assert!(params.app.is_none());
    assert!(params.app_args.is_none());
    assert!(params.caller_id.is_none());
    assert!(params.timeout.is_none());
}

// ── additional edge case tests ──────────────────────────────────────────────

#[test]
fn deserialize_stasis_start_with_replace_channel() {
    let json = r#"{
        "type": "StasisStart",
        "channel": {"id": "new-ch", "name": "PJSIP/100", "state": "Ring"},
        "args": [],
        "replace_channel": {"id": "old-ch", "name": "PJSIP/100", "state": "Up"}
    }"#;
    let event: AriEvent = serde_json::from_str(json).expect("deser");
    match event {
        AriEvent::StasisStart {
            replace_channel, ..
        } => {
            let rc = replace_channel.expect("replace_channel should be present");
            assert_eq!(rc.id, "old-ch");
        }
        other => panic!("expected StasisStart, got {other:?}"),
    }
}

#[test]
fn deserialize_bridge_blind_transfer_with_optionals() {
    let json = r#"{
        "type": "BridgeBlindTransfer",
        "channel": {"id": "ch1", "name": "PJSIP/100", "state": "Up"},
        "exten": "300",
        "context": "internal",
        "result": "Fail",
        "is_external": true,
        "bridge": {"id": "br1", "technology": "simple_bridge", "bridge_type": "mixing"},
        "transferee": {"id": "xfee", "name": "PJSIP/200", "state": "Up"},
        "replace_channel": {"id": "repl", "name": "PJSIP/300", "state": "Ring"}
    }"#;
    let event: AriEvent = serde_json::from_str(json).expect("deser");
    match event {
        AriEvent::BridgeBlindTransfer {
            bridge,
            transferee,
            replace_channel,
            is_external,
            ..
        } => {
            assert!(is_external);
            assert!(bridge.is_some());
            assert!(transferee.is_some());
            assert!(replace_channel.is_some());
        }
        other => panic!("expected BridgeBlindTransfer, got {other:?}"),
    }
}

#[test]
fn deserialize_channel_transfer_with_refer_to() {
    let json = r#"{
        "type": "ChannelTransfer",
        "channel": {"id": "ch1", "name": "PJSIP/100", "state": "Up"},
        "refer_to": {
            "destination_channel": {"id": "d1", "name": "PJSIP/200", "state": "Ring"}
        },
        "referred_by": {
            "source_channel": {"id": "s1", "name": "PJSIP/100", "state": "Up"}
        }
    }"#;
    let event: AriEvent = serde_json::from_str(json).expect("deser");
    match event {
        AriEvent::ChannelTransfer {
            refer_to,
            referred_by,
            ..
        } => {
            assert!(refer_to.is_some());
            assert!(referred_by.is_some());
        }
        other => panic!("expected ChannelTransfer, got {other:?}"),
    }
}

#[test]
fn deserialize_bridge_attended_transfer_with_optionals() {
    let json = r#"{
        "type": "BridgeAttendedTransfer",
        "transferer_first_leg": {"id": "leg1", "name": "PJSIP/100", "state": "Up"},
        "transferer_second_leg": {"id": "leg2", "name": "PJSIP/200", "state": "Up"},
        "result": "Success",
        "destination_type": "bridge",
        "is_external": false,
        "transferee": {"id": "xfee", "name": "PJSIP/300", "state": "Up"},
        "transfer_target": {"id": "tgt", "name": "PJSIP/400", "state": "Up"},
        "transferer_first_leg_bridge": {"id": "br1", "technology": "softmix", "bridge_type": "mixing"},
        "transferer_second_leg_bridge": {"id": "br2", "technology": "softmix", "bridge_type": "mixing"},
        "destination_bridge": "br-dest"
    }"#;
    let event: AriEvent = serde_json::from_str(json).expect("deser");
    match event {
        AriEvent::BridgeAttendedTransfer {
            transferee,
            transfer_target,
            transferer_first_leg_bridge,
            transferer_second_leg_bridge,
            destination_bridge,
            ..
        } => {
            assert!(transferee.is_some());
            assert!(transfer_target.is_some());
            assert!(transferer_first_leg_bridge.is_some());
            assert!(transferer_second_leg_bridge.is_some());
            assert_eq!(destination_bridge.as_deref(), Some("br-dest"));
        }
        other => panic!("expected BridgeAttendedTransfer, got {other:?}"),
    }
}

#[test]
fn ari_message_flattens_event() {
    // verify that serde(flatten) on the event field works correctly
    let json = r#"{
        "type": "BridgeCreated",
        "application": "demo",
        "timestamp": "2024-06-01T00:00:00.000+0000",
        "bridge": {"id": "br-1", "technology": "softmix", "bridge_type": "mixing"}
    }"#;
    let msg: AriMessage = serde_json::from_str(json).expect("deser");
    assert_eq!(msg.application, "demo");
    assert!(matches!(msg.event, AriEvent::BridgeCreated { .. }));
}

#[test]
fn contact_info_no_roundtrip() {
    let json = r#"{"uri": "sip:x@y", "contact_status": "Unknown", "aor": "x"}"#;
    let ci: ContactInfo = serde_json::from_str(json).expect("deser");
    assert!(ci.roundtrip_usec.is_none());
}

#[test]
fn peer_minimal() {
    let json = r#"{"peer_status": "Unreachable"}"#;
    let p: Peer = serde_json::from_str(json).expect("deser");
    assert_eq!(p.peer_status, "Unreachable");
    assert!(p.address.is_none());
    assert!(p.port.is_none());
    assert!(p.cause.is_none());
    assert!(p.time.is_none());
}

#[test]
fn endpoint_no_channels() {
    let json = r#"{"technology": "IAX2", "resource": "trunk"}"#;
    let ep: Endpoint = serde_json::from_str(json).expect("deser");
    assert!(ep.state.is_none());
    assert!(ep.channel_ids.is_empty());
}

#[test]
fn bridge_video_source_changed_no_old() {
    let json = format!(
        r#"{{"type": "BridgeVideoSourceChanged", "bridge": {}}}"#,
        minimal_bridge_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::BridgeVideoSourceChanged {
            old_video_source_id,
            ..
        } => {
            assert!(old_video_source_id.is_none());
        }
        other => panic!("expected BridgeVideoSourceChanged, got {other:?}"),
    }
}

#[test]
fn application_move_failed_no_args() {
    let json = format!(
        r#"{{"type": "ApplicationMoveFailed", "channel": {}, "destination": "app2"}}"#,
        minimal_channel_json()
    );
    let event: AriEvent = serde_json::from_str(&json).expect("deser");
    match event {
        AriEvent::ApplicationMoveFailed { args, .. } => {
            assert!(args.is_empty());
        }
        other => panic!("expected ApplicationMoveFailed, got {other:?}"),
    }
}
