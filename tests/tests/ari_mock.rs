mod common;
mod mock;

use std::time::Duration;

use asterisk_rs_ari::config::AriConfigBuilder;
use asterisk_rs_ari::{AriClient, AriError};
use asterisk_rs_core::config::ReconnectPolicy;

use mock::ari_server::MockAriServerBuilder;

/// build an ARI client pointed at the mock server
async fn connect_to_mock(port: u16) -> AriClient {
    let config = AriConfigBuilder::new("test-app")
        .host("127.0.0.1")
        .port(port)
        .username("testuser")
        .password("testpass")
        .reconnect(ReconnectPolicy::none())
        .build()
        .expect("failed to build ari config");

    AriClient::connect(config)
        .await
        .expect("failed to connect ari client")
}

#[tokio::test]
async fn connect_and_disconnect() {
    common::init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;
    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn get_request() {
    common::init_tracing();

    let server = MockAriServerBuilder::new()
        .route(
            "GET",
            "/ari/asterisk/info",
            200,
            r#"{"status":"Fully Booted"}"#,
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let info: serde_json::Value = client
        .get("asterisk/info")
        .await
        .expect("GET asterisk/info failed");

    assert_eq!(
        info["status"], "Fully Booted",
        "expected status field in response"
    );

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn post_request() {
    common::init_tracing();

    let body = r#"{"id":"chan-1","name":"SIP/100-0001","state":"Ring"}"#;
    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let channel: serde_json::Value = client
        .post("channels", &serde_json::json!({"endpoint": "SIP/100"}))
        .await
        .expect("POST channels failed");

    assert_eq!(channel["id"], "chan-1", "expected channel id in response");

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn delete_request() {
    common::init_tracing();

    let server = MockAriServerBuilder::new()
        .route("DELETE", "/ari/channels/chan-1", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    client
        .delete("channels/chan-1")
        .await
        .expect("DELETE channels/chan-1 failed");

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn api_error_handling() {
    common::init_tracing();

    let server = MockAriServerBuilder::new()
        .route(
            "GET",
            "/ari/channels/nonexistent",
            404,
            r#"{"message":"Channel not found"}"#,
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let result = client
        .get::<serde_json::Value>("channels/nonexistent")
        .await;

    match result {
        Err(AriError::Api { status, message }) => {
            assert_eq!(status, 404, "expected 404 status");
            assert!(
                message.contains("Channel not found"),
                "expected error body, got: {message}"
            );
        }
        Err(other) => panic!("expected AriError::Api, got: {other:?}"),
        Ok(val) => panic!("expected error, got success: {val:?}"),
    }

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn websocket_events() {
    common::init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;
    let mut sub = client.subscribe();

    // give the background ws task time to connect
    tokio::time::sleep(Duration::from_millis(200)).await;

    let event_json = r#"{
        "type": "StasisStart",
        "application": "test-app",
        "timestamp": "2024-01-01T00:00:00.000+0000",
        "channel": {
            "id": "chan-1",
            "name": "SIP/100-0001",
            "state": "Ring",
            "caller": { "name": "Test", "number": "100" },
            "connected": { "name": "", "number": "" },
            "dialplan": { "context": "default", "exten": "100", "priority": 1 }
        },
        "args": []
    }"#;

    server.send_event(event_json);

    let event = tokio::time::timeout(Duration::from_secs(5), sub.recv())
        .await
        .expect("timed out waiting for ws event")
        .expect("event subscription closed unexpectedly");

    assert_eq!(event.application, "test-app", "expected application field");
    match &event.event {
        asterisk_rs_ari::AriEvent::StasisStart { channel, .. } => {
            assert_eq!(channel.id, "chan-1", "expected channel id");
            assert_eq!(channel.state, "Ring", "expected channel state");
        }
        other => panic!("expected StasisStart, got: {other:?}"),
    }

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn unregistered_route_returns_404() {
    common::init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;

    let result = client.get::<serde_json::Value>("does/not/exist").await;

    match result {
        Err(AriError::Api { status, .. }) => {
            assert_eq!(status, 404, "expected 404 for unregistered route");
        }
        Err(other) => panic!("expected AriError::Api 404, got: {other:?}"),
        Ok(val) => panic!("expected error, got success: {val:?}"),
    }

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn put_request() {
    common::init_tracing();

    let body = r#"{"id":"bridge-1","bridge_type":"mixing"}"#;
    let server = MockAriServerBuilder::new()
        .route("PUT", "/ari/bridges/bridge-1", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let bridge: serde_json::Value = client
        .put("bridges/bridge-1", &serde_json::json!({"type": "mixing"}))
        .await
        .expect("PUT bridges should succeed");

    assert_eq!(bridge["id"], "bridge-1");
    assert_eq!(bridge["bridge_type"], "mixing");

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn filtered_subscription() {
    common::init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;

    // only accept StasisStart events
    let mut filtered = client.subscribe_filtered(|msg| {
        matches!(msg.event, asterisk_rs_ari::AriEvent::StasisStart { .. })
    });

    tokio::time::sleep(Duration::from_millis(200)).await;

    // send a ChannelDestroyed event first (should be filtered out)
    server.send_event(
        r#"{
        "type": "ChannelDestroyed",
        "application": "test-app",
        "timestamp": "2024-01-01T00:00:00.000+0000",
        "channel": {
            "id": "chan-99",
            "name": "SIP/200-0099",
            "state": "Down",
            "caller": {"name": "", "number": ""},
            "connected": {"name": "", "number": ""},
            "dialplan": {"context": "default", "exten": "s", "priority": 1}
        },
        "cause": 16,
        "cause_txt": "Normal"
    }"#,
    );

    // send a StasisStart event (should pass the filter)
    server.send_event(
        r#"{
        "type": "StasisStart",
        "application": "test-app",
        "timestamp": "2024-01-01T00:00:01.000+0000",
        "channel": {
            "id": "chan-42",
            "name": "SIP/100-0042",
            "state": "Ring",
            "caller": {"name": "Test", "number": "100"},
            "connected": {"name": "", "number": ""},
            "dialplan": {"context": "default", "exten": "100", "priority": 1}
        },
        "args": []
    }"#,
    );

    let event = tokio::time::timeout(Duration::from_secs(5), filtered.recv())
        .await
        .expect("timed out waiting for filtered event")
        .expect("subscription closed");

    // should receive StasisStart, not ChannelDestroyed
    match &event.event {
        asterisk_rs_ari::AriEvent::StasisStart { channel, .. } => {
            assert_eq!(channel.id, "chan-42");
        }
        other => panic!("expected StasisStart, got: {other:?}"),
    }

    client.disconnect();
    server.shutdown();
}
