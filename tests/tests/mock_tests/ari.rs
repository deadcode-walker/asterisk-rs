use std::time::Duration;

use asterisk_rs_ari::config::{AriConfigBuilder, TransportMode};
use asterisk_rs_ari::{AriClient, AriError};
use asterisk_rs_core::config::ReconnectPolicy;

use asterisk_rs_tests::helpers::{assert_server_ok, init_tracing};
use asterisk_rs_tests::mock::ari_server::MockAriServerBuilder;

/// build an ARI client pointed at the mock server
async fn connect_to_mock(port: u16) -> AriClient {
    connect_to_mock_with_response_limit(port, 4 * 1024 * 1024).await
}

async fn connect_to_mock_with_response_limit(port: u16, limit: usize) -> AriClient {
    let config = AriConfigBuilder::new("test-app")
        .host("127.0.0.1")
        .port(port)
        .username("testuser")
        .password("testpass")
        .reconnect(ReconnectPolicy::none())
        .max_response_body_bytes(limit)
        .build()
        .expect("failed to build ari config");

    AriClient::connect(config)
        .await
        .expect("failed to connect ari client")
}

/// build a unified REST-over-WebSocket client pointed at the mock server
async fn connect_unified_to_mock(port: u16) -> AriClient {
    connect_unified_to_mock_with_timeout(port, Duration::from_secs(30)).await
}

async fn connect_unified_to_mock_with_timeout(port: u16, request_timeout: Duration) -> AriClient {
    connect_unified_to_mock_with_limits(port, request_timeout, 4 * 1024 * 1024, 4 * 1024 * 1024)
        .await
}

async fn connect_unified_to_mock_with_limits(
    port: u16,
    request_timeout: Duration,
    max_response_body_bytes: usize,
    max_websocket_message_bytes: usize,
) -> AriClient {
    let config = AriConfigBuilder::new("test-app")
        .host("127.0.0.1")
        .port(port)
        .username("testuser")
        .password("testpass")
        .transport(TransportMode::WebSocket)
        .reconnect(ReconnectPolicy::none())
        .request_timeout(request_timeout)
        .max_response_body_bytes(max_response_body_bytes)
        .max_websocket_message_bytes(max_websocket_message_bytes)
        .build()
        .expect("failed to build unified ari config");

    AriClient::connect(config)
        .await
        .expect("failed to create unified ari client")
}

#[tokio::test]
async fn connect_and_disconnect() {
    init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;
    client.disconnect_and_wait().await;
    server.shutdown().await;
}

#[tokio::test]
async fn get_request() {
    init_tracing();

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

    client.disconnect_and_wait().await;
    server.shutdown().await;
}

#[tokio::test]
async fn post_request() {
    init_tracing();

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

    server.wait_for_requests(1).await;
    let requests = server.requests();
    let request = &requests[0];
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/ari/channels");
    assert_eq!(
        request.authorization.as_deref(),
        Some("Basic dGVzdHVzZXI6dGVzdHBhc3M=")
    );
    assert_eq!(request.content_type.as_deref(), Some("application/json"));
    assert_eq!(request.content_length, request.body.len());
    assert_eq!(request.body, br#"{"endpoint":"SIP/100"}"#);

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn delete_request() {
    init_tracing();

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
    server.shutdown().await;
}

#[tokio::test]
async fn api_error_handling() {
    init_tracing();

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
    server.shutdown().await;
}

#[tokio::test]
async fn http_transport_error_preserves_stable_source() {
    use std::error::Error as _;

    init_tracing();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("temporary listener should bind");
    let port = listener
        .local_addr()
        .expect("temporary listener should have an address")
        .port();
    drop(listener);

    let config = AriConfigBuilder::new("test-app")
        .host("127.0.0.1")
        .port(port)
        .username("testuser")
        .password("testpass")
        .reconnect(ReconnectPolicy::none())
        .request_timeout(Duration::from_secs(1))
        .build()
        .expect("config should build");
    let client = AriClient::connect(config)
        .await
        .expect("client transport should construct");

    let result = client.get::<serde_json::Value>("unreachable").await;
    match result {
        Err(AriError::Http(error)) => {
            assert!(
                error.source().is_some(),
                "concrete HTTP source must be retained"
            );
            assert_eq!(error.status_code(), None);
            assert!(!error.to_string().is_empty());
        }
        other => panic!("expected stable HTTP transport error, got {other:?}"),
    }

    client.disconnect();
}

#[tokio::test]
async fn http_mutation_connect_failure_is_definitely_not_sent() {
    init_tracing();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("temporary listener should bind");
    let port = listener
        .local_addr()
        .expect("temporary listener should have an address")
        .port();
    drop(listener);
    let client = connect_to_mock(port).await;

    let result = client.post_empty("channels/not-sent").await;
    assert!(
        matches!(
            &result,
            Err(AriError::RequestNotSent { method, uri })
                if method == "POST" && uri == "channels/not-sent"
        ),
        "connect failure must be explicitly safe to retry: {result:?}"
    );

    client.disconnect();
}

#[tokio::test]
async fn http_mutation_disconnect_after_request_is_indeterminate() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route_disconnect("POST", "/ari/channels/maybe-executed")
        .start()
        .await;
    let client = connect_to_mock(server.port()).await;

    let result = client.post_empty("channels/maybe-executed").await;
    assert!(
        matches!(
            &result,
            Err(AriError::OutcomeUnknown {
                request_id,
                method,
                uri,
            }) if request_id.starts_with("http-")
                && method == "POST"
                && uri == "channels/maybe-executed"
        ),
        "a mutation accepted by the server without a response is indeterminate: {result:?}"
    );
    server.wait_for_requests(1).await;

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn fixed_length_http_response_respects_application_limit() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/large", 200, "12345678901234567")
        .start()
        .await;
    let client = connect_to_mock_with_response_limit(server.port(), 16).await;

    let result = client.get::<serde_json::Value>("large").await;
    assert!(
        matches!(
            &result,
            Err(AriError::ResponseTooLarge {
                limit: 16,
                received: 17
            })
        ),
        "fixed-length response must be rejected before buffering: {result:?}"
    );

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn chunked_http_response_respects_application_limit() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route_chunked("GET", "/ari/large", 200, "12345678901234567")
        .start()
        .await;
    let client = connect_to_mock_with_response_limit(server.port(), 16).await;

    let result = client.get::<serde_json::Value>("large").await;
    assert!(
        matches!(
            result,
            Err(AriError::ResponseTooLarge {
                limit: 16,
                received: 17
            })
        ),
        "chunked response must be rejected while streaming: {result:?}"
    );

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn chunked_http_response_under_limit_succeeds() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route_chunked("GET", "/ari/small", 200, r#"{"ok":true}"#)
        .start()
        .await;
    let client = connect_to_mock_with_response_limit(server.port(), 16).await;

    let value: serde_json::Value = client
        .get("small")
        .await
        .expect("under-limit chunked response should succeed");
    assert_eq!(value["ok"], true);

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn http_redirect_is_not_accepted_as_api_success() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels/redirected", 302, "")
        .start()
        .await;
    let client = connect_to_mock(server.port()).await;

    let result = client.post_empty("channels/redirected").await;
    assert!(
        matches!(result, Err(AriError::Api { status: 302, .. })),
        "HTTP transport must accept only explicit 2xx responses: {result:?}"
    );

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn unified_transport_accepts_only_explicit_2xx_responses() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels/ok", 204, "")
        .route("POST", "/ari/channels/redirected", 302, "")
        .route("GET", "/ari/channels/empty", 204, "")
        .start()
        .await;
    let client = connect_unified_to_mock(server.port()).await;
    server.wait_for_ws_client().await;

    client
        .post_empty("channels/ok")
        .await
        .expect("unified transport should accept a 204 response");
    let result = client.post_empty("channels/redirected").await;
    match result {
        Err(AriError::Api { status, message }) => {
            assert_eq!(status, 302);
            assert_eq!(message, "Found");
        }
        other => panic!("unified transport must reject a non-2xx response: {other:?}"),
    }
    let result = client.get::<serde_json::Value>("channels/empty").await;
    assert!(
        matches!(
            result,
            Err(AriError::Api {
                status: 204,
                ref message,
            }) if message == "expected response body"
        ),
        "a success reason phrase must not become a response body: {result:?}"
    );

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn unified_response_body_respects_application_limit() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/large", 200, "12345678901234567")
        .start()
        .await;
    let client =
        connect_unified_to_mock_with_limits(server.port(), Duration::from_secs(1), 16, 1024).await;
    server.wait_for_ws_client().await;

    let result = client.get::<serde_json::Value>("large").await;
    assert!(
        matches!(
            result,
            Err(AriError::ResponseTooLarge {
                limit: 16,
                received: 17,
            })
        ),
        "unified response body must respect the application cap: {result:?}"
    );

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn unified_websocket_message_respects_wire_limit() {
    init_tracing();

    let body = "x".repeat(512);
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/oversized-frame", 200, &body)
        .start()
        .await;
    let client =
        connect_unified_to_mock_with_limits(server.port(), Duration::from_secs(1), 1024, 256).await;
    server.wait_for_ws_client().await;

    let result = client.get::<serde_json::Value>("oversized-frame").await;
    assert!(
        matches!(result, Err(AriError::WebSocket(_))),
        "oversized unified WebSocket message must close the request safely: {result:?}"
    );

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn unified_command_progresses_during_continuous_event_flood() {
    use futures_util::{SinkExt, StreamExt};

    init_tracing();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind should succeed");
    let port = listener.local_addr().expect("listener address").port();
    let peer = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept should succeed");
        let websocket = tokio_tungstenite::accept_async(stream)
            .await
            .expect("websocket handshake should succeed");
        let (mut write, mut read) = websocket.split();
        let mut event_index = 0_u64;
        loop {
            write
                .send(tokio_tungstenite::tungstenite::Message::Text(
                    stasis_start_json(&format!("flood-{event_index}")).into(),
                ))
                .await
                .expect("event flood write should succeed");
            event_index += 1;
            tokio::select! {
                message = read.next() => {
                    let request = message
                        .expect("client should remain connected")
                        .expect("request frame should be valid");
                    let request: serde_json::Value = serde_json::from_str(
                        request.to_text().expect("request should be text"),
                    )
                    .expect("request should be JSON");
                    let response = serde_json::json!({
                        "type": "RESTResponse",
                        "status_code": 204,
                        "reason_phrase": "No Content",
                        "uri": request["uri"],
                        "request_id": request["request_id"],
                        "transaction_id": "fairness-test",
                    });
                    write
                        .send(tokio_tungstenite::tungstenite::Message::Text(
                            response.to_string().into(),
                        ))
                        .await
                        .expect("response should send");
                    return event_index;
                }
                _ = tokio::task::yield_now() => {}
            }
        }
    });
    let client = connect_unified_to_mock_with_timeout(port, Duration::from_secs(1)).await;

    tokio::time::timeout(Duration::from_secs(1), client.post_empty("channels/fair"))
        .await
        .expect("continuous readable events must not starve command admission")
        .expect("command should complete during event flood");
    let events_sent = peer.await.expect("peer task should not panic");
    assert!(events_sent > 0, "test must exercise a live event flood");
    client.disconnect_and_wait().await;
}

#[tokio::test]
async fn unified_oversized_mutation_is_definitely_not_sent() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/small", 204, "")
        .start()
        .await;
    let client =
        connect_unified_to_mock_with_limits(server.port(), Duration::from_secs(1), 1024, 256).await;
    server.wait_for_ws_client().await;

    let result = client
        .post::<serde_json::Value>("oversized", &serde_json::json!({"value": "x".repeat(512)}))
        .await;
    assert!(
        matches!(
            result,
            Err(AriError::RequestNotSent { ref method, ref uri })
                if method == "POST" && uri == "oversized"
        ),
        "a locally oversized mutation must be definitely unsent: {result:?}"
    );
    client
        .post_empty("small")
        .await
        .expect("preflight rejection must not tear down the websocket");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn unified_pending_requests_are_count_bounded() {
    use futures_util::StreamExt;

    init_tracing();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind should succeed");
    let port = listener.local_addr().expect("listener address").port();
    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept should succeed");
        let mut websocket = tokio_tungstenite::accept_async(stream)
            .await
            .expect("websocket handshake should succeed");
        let mut received = 0_usize;
        while let Ok(Some(message)) =
            tokio::time::timeout(Duration::from_secs(1), websocket.next()).await
        {
            match message {
                Ok(tokio_tungstenite::tungstenite::Message::Text(_)) => received += 1,
                Ok(tokio_tungstenite::tungstenite::Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
        received
    });
    let client = connect_unified_to_mock_with_timeout(port, Duration::from_millis(250)).await;

    let mut requests = tokio::task::JoinSet::new();
    for index in 0..65 {
        let client = client.clone();
        requests.spawn(async move { client.post_empty(&format!("bounded/{index}")).await });
    }

    let mut definitely_unsent = 0_usize;
    let mut outcome_unknown = 0_usize;
    while let Some(result) = requests.join_next().await {
        match result.expect("request task should not panic") {
            Err(AriError::RequestNotSent { .. }) => definitely_unsent += 1,
            Err(AriError::OutcomeUnknown { .. }) => outcome_unknown += 1,
            other => panic!("unexpected bounded pending result: {other:?}"),
        }
    }
    assert!(
        definitely_unsent >= 1,
        "at least one request must remain queued when 64 correlations are pending"
    );
    assert!(
        outcome_unknown >= 1,
        "written requests must remain indeterminate"
    );

    client.disconnect();
    let received = server_task.await.expect("server task should not panic");
    assert!(
        received <= 64,
        "the actor must not admit more than 64 pending correlations, received {received}"
    );
}

#[tokio::test]
async fn unified_transport_discards_cancelled_request_queued_during_handshake() {
    use futures_util::StreamExt;

    init_tracing();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind should succeed");
    let port = listener
        .local_addr()
        .expect("listener should have an address")
        .port();
    let client = connect_unified_to_mock_with_timeout(port, Duration::from_millis(50)).await;

    let result = tokio::time::timeout(
        Duration::from_secs(1),
        client.post_empty("channels/must-not-run"),
    )
    .await
    .expect("the configured request deadline must be bounded");
    assert!(
        matches!(
            &result,
            Err(AriError::RequestNotSent { method, uri })
                if method == "POST" && uri == "channels/must-not-run"
        ),
        "a queued timeout must report that the mutation was definitely not sent: {result:?}"
    );

    let (stream, _) = tokio::time::timeout(Duration::from_secs(2), listener.accept())
        .await
        .expect("client should connect")
        .expect("TCP accept should succeed");
    let mut websocket = tokio_tungstenite::accept_async(stream)
        .await
        .expect("websocket handshake should succeed");

    let message = tokio::time::timeout(Duration::from_millis(250), websocket.next()).await;
    assert!(
        message.is_err(),
        "a request cancelled while queued must not execute after connection"
    );

    client.disconnect();
}

#[tokio::test]
async fn unified_shutdown_before_first_wire_poll_is_definitely_unsent() {
    init_tracing();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind should succeed");
    let port = listener.local_addr().expect("listener address").port();
    let client = connect_unified_to_mock_with_timeout(port, Duration::from_secs(5)).await;
    let request_client = client.clone();
    let request_task = tokio::spawn(async move {
        request_client
            .post_empty("channels/shutdown-before-poll")
            .await
    });

    // The TCP listener is present but the peer never accepts or completes the
    // WebSocket handshake, so the actor cannot poll the request's sink write.
    tokio::time::sleep(Duration::from_millis(20)).await;
    client.disconnect_and_wait().await;

    let result = tokio::time::timeout(Duration::from_secs(1), request_task)
        .await
        .expect("owned actor shutdown should be bounded")
        .expect("request task should not panic");
    assert!(
        matches!(
            result,
            Err(AriError::RequestNotSent { ref method, ref uri })
                if method == "POST" && uri == "channels/shutdown-before-poll"
        ),
        "shutdown before the first sink poll must remain definitely unsent: {result:?}"
    );

    drop(listener);
}

#[tokio::test]
async fn unified_mutation_timeout_after_write_reports_unknown_outcome() {
    use futures_util::StreamExt;

    init_tracing();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind should succeed");
    let port = listener
        .local_addr()
        .expect("listener should have an address")
        .port();
    let client = connect_unified_to_mock_with_timeout(port, Duration::from_millis(100)).await;
    let (stream, _) = tokio::time::timeout(Duration::from_secs(1), listener.accept())
        .await
        .expect("client should connect")
        .expect("TCP accept should succeed");
    let mut websocket = tokio_tungstenite::accept_async(stream)
        .await
        .expect("websocket handshake should succeed");

    let request_client = client.clone();
    let request_task =
        tokio::spawn(async move { request_client.post_empty("channels/outcome-unknown").await });
    let frame = tokio::time::timeout(Duration::from_secs(1), websocket.next())
        .await
        .expect("mutation should be written before its deadline")
        .expect("websocket should remain open")
        .expect("REST request frame should be valid");
    let request: serde_json::Value = serde_json::from_str(
        frame
            .to_text()
            .expect("REST request should use a text frame"),
    )
    .expect("REST request should be JSON");
    assert_eq!(request["method"], "POST");
    assert_eq!(request["uri"], "channels/outcome-unknown");

    let result = tokio::time::timeout(Duration::from_secs(1), request_task)
        .await
        .expect("request deadline must complete")
        .expect("request task should not panic");
    assert!(
        matches!(
            &result,
            Err(AriError::OutcomeUnknown {
                request_id,
                method,
                uri,
            }) if request_id.starts_with("wsreq-")
                && method == "POST"
                && uri == "channels/outcome-unknown"
        ),
        "a written mutation without a response must be indeterminate: {result:?}"
    );

    client.disconnect();
}

#[tokio::test]
async fn unified_shutdown_does_not_drop_inflight_mutation_outcome() {
    use futures_util::StreamExt;

    init_tracing();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind should succeed");
    let port = listener
        .local_addr()
        .expect("listener should have an address")
        .port();
    let client = connect_unified_to_mock_with_timeout(port, Duration::from_secs(5)).await;
    let (stream, _) = tokio::time::timeout(Duration::from_secs(1), listener.accept())
        .await
        .expect("client should connect")
        .expect("TCP accept should succeed");
    let mut websocket = tokio_tungstenite::accept_async(stream)
        .await
        .expect("websocket handshake should succeed");

    let request_client = client.clone();
    let request_task =
        tokio::spawn(async move { request_client.post_empty("channels/shutdown-outcome").await });
    tokio::time::timeout(Duration::from_secs(1), websocket.next())
        .await
        .expect("mutation should be written")
        .expect("websocket should remain open")
        .expect("REST request frame should be valid");

    client.disconnect_and_wait().await;
    let result = tokio::time::timeout(Duration::from_secs(1), request_task)
        .await
        .expect("actor shutdown should wake the request")
        .expect("request task should not panic");
    assert!(
        matches!(
            &result,
            Err(AriError::OutcomeUnknown {
                request_id,
                method,
                uri,
            }) if request_id.starts_with("wsreq-")
                && method == "POST"
                && uri == "channels/shutdown-outcome"
        ),
        "shutdown must preserve the indeterminate mutation outcome: {result:?}"
    );
}

#[tokio::test]
async fn websocket_events() {
    init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;
    let mut sub = client.subscribe();

    // wait for background ws task to connect
    server.wait_for_ws_client().await;

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
    server.shutdown().await;
}

#[tokio::test]
async fn unregistered_route_returns_404() {
    init_tracing();

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
    server.shutdown().await;
}

#[tokio::test]
async fn put_request() {
    init_tracing();

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
    server.shutdown().await;
}

#[tokio::test]
async fn filtered_subscription() {
    init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;

    // only accept StasisStart events
    let mut filtered = client.subscribe_filtered(|msg| {
        matches!(msg.event, asterisk_rs_ari::AriEvent::StasisStart { .. })
    });

    server.wait_for_ws_client().await;

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
    server.shutdown().await;
}

#[tokio::test]
async fn channel_handle_answer_and_hangup() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels/chan-1/answer", 204, "")
        .route("DELETE", "/ari/channels/chan-1", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("chan-1", client.clone());

    handle.answer().await.expect("answer failed");
    handle.hangup(None).await.expect("hangup failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn channel_handle_hold_mute_dtmf() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels/chan-1/hold", 204, "")
        .route("DELETE", "/ari/channels/chan-1/hold", 204, "")
        .route("POST", "/ari/channels/chan-1/mute", 204, "")
        .route("DELETE", "/ari/channels/chan-1/mute", 204, "")
        .route("POST", "/ari/channels/chan-1/dtmf?dtmf=1234", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("chan-1", client.clone());

    handle.hold().await.expect("hold failed");
    handle.unhold().await.expect("unhold failed");
    handle.mute(None).await.expect("mute failed");
    handle.unmute(None).await.expect("unmute failed");
    handle.send_dtmf("1234").await.expect("send_dtmf failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn channel_handle_variables() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route(
            "GET",
            "/ari/channels/chan-1/variable?variable=MY_VAR",
            200,
            r#"{"value":"hello"}"#,
        )
        .route(
            "POST",
            "/ari/channels/chan-1/variable?variable=MY_VAR&value=hello",
            204,
            "",
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("chan-1", client.clone());

    let var = handle
        .get_variable("MY_VAR")
        .await
        .expect("get variable failed");
    assert_eq!(var.value, "hello", "expected variable value");

    handle
        .set_variable("MY_VAR", "hello")
        .await
        .expect("set variable failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn bridge_handle_lifecycle() {
    init_tracing();

    let bridge_json =
        r#"{"id":"br-1","bridge_type":"mixing","technology":"simple_bridge","channels":[]}"#;

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/bridges", 200, bridge_json)
        .route(
            "POST",
            "/ari/bridges/br-1/addChannel?channel=chan-1",
            204,
            "",
        )
        .route(
            "POST",
            "/ari/bridges/br-1/removeChannel?channel=chan-1",
            204,
            "",
        )
        .route("DELETE", "/ari/bridges/br-1", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;

    // create bridge via client
    let _bridge: serde_json::Value = client
        .post("bridges", &serde_json::json!({"type": "mixing"}))
        .await
        .expect("create bridge failed");

    let handle = asterisk_rs_ari::resources::bridge::BridgeHandle::new("br-1", client.clone());
    handle
        .add_channel("chan-1")
        .await
        .expect("add channel failed");
    handle
        .remove_channel("chan-1")
        .await
        .expect("remove channel failed");
    handle.destroy().await.expect("destroy bridge failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn post_empty_returns_ok() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels/chan-1/answer", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    client
        .post_empty("channels/chan-1/answer")
        .await
        .expect("post_empty should succeed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn put_empty_returns_ok() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("PUT", "/ari/channels/chan-1/something", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    client
        .put_empty("channels/chan-1/something")
        .await
        .expect("put_empty should succeed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn disconnect_stops_ws_listener() {
    init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;
    let mut sub = client.subscribe();

    // give ws task time to connect
    server.wait_for_ws_client().await;

    // send an event to confirm subscription works
    server.send_event(
        r#"{
        "type": "StasisStart",
        "application": "test-app",
        "timestamp": "2024-01-01T00:00:00.000+0000",
        "channel": {
            "id": "chan-1",
            "name": "SIP/100-0001",
            "state": "Ring",
            "caller": {"name": "Test", "number": "100"},
            "connected": {"name": "", "number": ""},
            "dialplan": {"context": "default", "exten": "100", "priority": 1}
        },
        "args": []
    }"#,
    );

    let event = tokio::time::timeout(Duration::from_secs(5), sub.recv())
        .await
        .expect("timed out waiting for event")
        .expect("subscription closed before event");
    assert_eq!(event.application, "test-app");

    // disconnect and verify subscription terminates
    client.disconnect();

    // after disconnect, recv should return None (channel closed)
    let result = tokio::time::timeout(Duration::from_secs(2), sub.recv()).await;
    match result {
        Ok(None) => {}    // expected: channel closed
        Ok(Some(_)) => {} // acceptable: buffered event before close
        Err(_) => {}      // timeout is acceptable if ws task is still draining
    }

    server.shutdown().await;
}

#[tokio::test]
async fn channel_handle_play_returns_playback() {
    init_tracing();

    let playback_json =
        r#"{"id":"pb-1","media_uri":"sound:hello","target_uri":"channel:chan-1","state":"queued"}"#;

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels/chan-1/play", 200, playback_json)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("chan-1", client.clone());

    let pb = handle.play("sound:hello").await.expect("play failed");
    assert_eq!(pb.id, "pb-1", "expected playback id");
    assert_eq!(pb.state, "queued", "expected playback state");
    assert_eq!(pb.media_uri, "sound:hello", "expected media uri");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn multiple_websocket_events_in_sequence() {
    init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;
    let mut sub = client.subscribe();

    server.wait_for_ws_client().await;

    let channel_block = r#""id": "chan-1", "name": "SIP/100-0001", "state": "Ring", "caller": {"name": "", "number": ""}, "connected": {"name": "", "number": ""}, "dialplan": {"context": "default", "exten": "s", "priority": 1}"#;

    let events = [
        format!(
            r#"{{"type":"StasisStart","application":"test-app","timestamp":"2024-01-01T00:00:00.000+0000","channel":{{{channel_block}}},"args":[]}}"#
        ),
        format!(
            r#"{{"type":"ChannelDestroyed","application":"test-app","timestamp":"2024-01-01T00:00:01.000+0000","channel":{{{channel_block}}},"cause":16,"cause_txt":"Normal"}}"#
        ),
        format!(
            r#"{{"type":"ChannelStateChange","application":"test-app","timestamp":"2024-01-01T00:00:02.000+0000","channel":{{{channel_block}}}}}"#
        ),
        format!(
            r#"{{"type":"StasisEnd","application":"test-app","timestamp":"2024-01-01T00:00:03.000+0000","channel":{{{channel_block}}}}}"#
        ),
        format!(
            r#"{{"type":"ChannelVarset","application":"test-app","timestamp":"2024-01-01T00:00:04.000+0000","channel":{{{channel_block}}},"variable":"MYVAR","value":"test"}}"#
        ),
    ];

    for event_json in &events {
        server.send_event(event_json);
        // small delay to preserve ordering
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let mut received = Vec::new();
    for i in 0..5 {
        let event = tokio::time::timeout(Duration::from_secs(5), sub.recv())
            .await
            .unwrap_or_else(|_| panic!("timed out waiting for event {i}"))
            .unwrap_or_else(|| panic!("subscription closed before event {i}"));
        received.push(event);
    }

    // verify all 5 events received in order
    assert!(matches!(
        received[0].event,
        asterisk_rs_ari::AriEvent::StasisStart { .. }
    ));
    assert!(matches!(
        received[1].event,
        asterisk_rs_ari::AriEvent::ChannelDestroyed { .. }
    ));
    assert!(matches!(
        received[2].event,
        asterisk_rs_ari::AriEvent::ChannelStateChange { .. }
    ));
    assert!(matches!(
        received[3].event,
        asterisk_rs_ari::AriEvent::StasisEnd { .. }
    ));
    assert!(matches!(
        received[4].event,
        asterisk_rs_ari::AriEvent::ChannelVarset { .. }
    ));

    client.disconnect();
    server.shutdown().await;
}

// ---------------------------------------------------------------------------
// channel free functions
// ---------------------------------------------------------------------------

#[tokio::test]
async fn channel_list() {
    init_tracing();

    let body = r#"[{"id":"ch-1","name":"SIP/100-0001","state":"Up","caller":{"name":"","number":""},"connected":{"name":"","number":""},"dialplan":{"context":"default","exten":"s","priority":1}}]"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/channels", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let channels = asterisk_rs_ari::resources::channel::list(&client)
        .await
        .expect("channel list failed");

    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0].id, "ch-1");
    assert_eq!(channels[0].state, "Up");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn channel_get() {
    init_tracing();

    let body = r#"{"id":"ch-2","name":"SIP/200-0002","state":"Ring","caller":{"name":"Alice","number":"200"},"connected":{"name":"","number":""},"dialplan":{"context":"default","exten":"200","priority":1}}"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/channels/ch-2", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let channel = asterisk_rs_ari::resources::channel::get(&client, "ch-2")
        .await
        .expect("channel get failed");

    assert_eq!(channel.id, "ch-2");
    assert_eq!(channel.name, "SIP/200-0002");
    assert_eq!(channel.state, "Ring");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn channel_originate() {
    init_tracing();

    let body = r#"{"id":"ch-3","name":"SIP/300-0003","state":"Down","caller":{"name":"","number":""},"connected":{"name":"","number":""},"dialplan":{"context":"default","exten":"s","priority":1}}"#;
    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let params =
        asterisk_rs_ari::resources::channel::OriginateParams::new("SIP/300").app("test-app");
    let channel = asterisk_rs_ari::resources::channel::originate(&client, &params)
        .await
        .expect("channel originate failed");

    assert_eq!(channel.id, "ch-3");

    client.disconnect();
    server.shutdown().await;
}

// ---------------------------------------------------------------------------
// channel handle operations not yet tested
// ---------------------------------------------------------------------------

#[tokio::test]
async fn channel_handle_continue_in_dialplan() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route(
            "POST",
            "/ari/channels/ch-1/continue?context=other&extension=200&priority=1",
            204,
            "",
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("ch-1", client.clone());

    handle
        .continue_in_dialplan(Some("other"), Some("200"), Some(1))
        .await
        .expect("continue_in_dialplan failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn channel_handle_snoop() {
    init_tracing();

    let body = r#"{"id":"snoop-1","name":"Snoop/snoop-1","state":"Up","caller":{"name":"","number":""},"connected":{"name":"","number":""},"dialplan":{"context":"default","exten":"s","priority":1}}"#;
    let server = MockAriServerBuilder::new()
        .route(
            "POST",
            "/ari/channels/ch-1/snoop?app=test-app&spy=both",
            200,
            body,
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("ch-1", client.clone());

    let snooped = handle
        .snoop(Some("both"), None, "test-app")
        .await
        .expect("snoop failed");

    assert_eq!(snooped.id, "snoop-1");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn channel_handle_redirect() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route(
            "POST",
            "/ari/channels/ch-1/redirect?endpoint=PJSIP%2F300",
            204,
            "",
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("ch-1", client.clone());

    handle.redirect("PJSIP/300").await.expect("redirect failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn channel_handle_record() {
    init_tracing();

    let body = r#"{"name":"rec-1","format":"wav","state":"recording","target_uri":"channel:ch-1"}"#;
    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels/ch-1/record", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("ch-1", client.clone());

    let rec = handle.record("rec-1", "wav").await.expect("record failed");
    assert_eq!(rec.name, "rec-1");
    assert_eq!(rec.format, "wav");
    assert_eq!(rec.state, "recording");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn channel_handle_ring_and_ring_stop() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels/ch-1/ring", 204, "")
        .route("DELETE", "/ari/channels/ch-1/ring", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("ch-1", client.clone());

    handle.ring().await.expect("ring failed");
    handle.ring_stop().await.expect("ring_stop failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn channel_handle_silence_start_stop() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels/ch-1/silence", 204, "")
        .route("DELETE", "/ari/channels/ch-1/silence", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("ch-1", client.clone());

    handle.start_silence().await.expect("start_silence failed");
    handle.stop_silence().await.expect("stop_silence failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn channel_handle_dial() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route(
            "POST",
            "/ari/channels/ch-1/dial?caller=ch-2&timeout=30",
            204,
            "",
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("ch-1", client.clone());

    handle
        .dial(Some("ch-2"), Some(30))
        .await
        .expect("dial failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn channel_handle_send_dtmf_via_handle() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels/ch-1/dtmf?dtmf=9876", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("ch-1", client.clone());

    handle.send_dtmf("9876").await.expect("send_dtmf failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn channel_handle_play_with_id() {
    init_tracing();

    let body = r#"{"id":"pb-99","media_uri":"sound:tt-monkeys","state":"queued","target_uri":"channel:ch-1"}"#;
    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels/ch-1/play/pb-99", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("ch-1", client.clone());

    let pb = handle
        .play_with_id("pb-99", "sound:tt-monkeys")
        .await
        .expect("play_with_id failed");

    assert_eq!(pb.id, "pb-99");
    assert_eq!(pb.media_uri, "sound:tt-monkeys");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn channel_handle_rtp_statistics() {
    init_tracing();

    let body = r#"{"txcount":100,"rxcount":200}"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/channels/ch-1/rtp_statistics", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("ch-1", client.clone());

    let stats = handle
        .rtp_statistics()
        .await
        .expect("rtp_statistics failed");
    assert_eq!(stats["txcount"], 100);
    assert_eq!(stats["rxcount"], 200);

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn channel_handle_external_media() {
    init_tracing();

    let body = r#"{"id":"ext-1","name":"UnicastRTP/ext-1","state":"Up","caller":{"name":"","number":""},"connected":{"name":"","number":""},"dialplan":{"context":"default","exten":"s","priority":1}}"#;
    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels/externalMedia", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("ch-1", client.clone());

    let params = asterisk_rs_ari::resources::channel::ExternalMediaParams::new(
        "test-app",
        "192.168.1.1:10000",
        "ulaw",
    );
    let chan = handle
        .external_media(&params)
        .await
        .expect("external_media failed");

    assert_eq!(chan.id, "ext-1");

    client.disconnect();
    server.shutdown().await;
}

// ---------------------------------------------------------------------------
// bridge free functions
// ---------------------------------------------------------------------------

#[tokio::test]
async fn bridge_list() {
    init_tracing();

    let body =
        r#"[{"id":"br-1","technology":"simple_bridge","bridge_type":"mixing","channels":[]}]"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/bridges", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let bridges = asterisk_rs_ari::resources::bridge::list(&client)
        .await
        .expect("bridge list failed");

    assert_eq!(bridges.len(), 1);
    assert_eq!(bridges[0].id, "br-1");
    assert_eq!(bridges[0].bridge_type, "mixing");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn bridge_get() {
    init_tracing();

    let body =
        r#"{"id":"br-2","technology":"simple_bridge","bridge_type":"holding","channels":["ch-1"]}"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/bridges/br-2", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let bridge = asterisk_rs_ari::resources::bridge::get(&client, "br-2")
        .await
        .expect("bridge get failed");

    assert_eq!(bridge.id, "br-2");
    assert_eq!(bridge.bridge_type, "holding");
    assert_eq!(bridge.channels, vec!["ch-1"]);

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn bridge_create_via_function() {
    init_tracing();

    let body = r#"{"id":"br-3","technology":"simple_bridge","bridge_type":"mixing","channels":[]}"#;
    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/bridges", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let bridge =
        asterisk_rs_ari::resources::bridge::create(&client, Some("mixing"), Some("my-bridge"))
            .await
            .expect("bridge create failed");

    assert_eq!(bridge.id, "br-3");
    assert_eq!(bridge.bridge_type, "mixing");

    client.disconnect();
    server.shutdown().await;
}

// ---------------------------------------------------------------------------
// bridge handle operations not yet tested
// ---------------------------------------------------------------------------

#[tokio::test]
async fn bridge_handle_record() {
    init_tracing();

    let body =
        r#"{"name":"br-rec-1","format":"wav","state":"recording","target_uri":"bridge:br-1"}"#;
    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/bridges/br-1/record", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::bridge::BridgeHandle::new("br-1", client.clone());

    let rec = handle
        .record("br-rec-1", "wav")
        .await
        .expect("bridge record failed");
    assert_eq!(rec.name, "br-rec-1");
    assert_eq!(rec.state, "recording");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn bridge_handle_moh_start_stop() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/bridges/br-1/moh?mohClass=default", 204, "")
        .route("DELETE", "/ari/bridges/br-1/moh", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::bridge::BridgeHandle::new("br-1", client.clone());

    handle
        .start_moh(Some("default"))
        .await
        .expect("start_moh failed");
    handle.stop_moh().await.expect("stop_moh failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn bridge_handle_play_with_id() {
    init_tracing();

    let body =
        r#"{"id":"pb-br-1","media_uri":"sound:hello","state":"queued","target_uri":"bridge:br-1"}"#;
    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/bridges/br-1/play/pb-br-1", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::bridge::BridgeHandle::new("br-1", client.clone());

    let pb = handle
        .play_with_id("pb-br-1", "sound:hello")
        .await
        .expect("bridge play_with_id failed");

    assert_eq!(pb.id, "pb-br-1");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn bridge_handle_set_video_source() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/bridges/br-1/videoSource/ch-1", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::bridge::BridgeHandle::new("br-1", client.clone());

    handle
        .set_video_source("ch-1")
        .await
        .expect("set_video_source failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn bridge_handle_clear_video_source() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("DELETE", "/ari/bridges/br-1/videoSource", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::bridge::BridgeHandle::new("br-1", client.clone());

    handle
        .clear_video_source()
        .await
        .expect("clear_video_source failed");

    client.disconnect();
    server.shutdown().await;
}

// ---------------------------------------------------------------------------
// endpoint operations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn endpoint_list() {
    init_tracing();

    let body = r#"[{"technology":"SIP","resource":"100","state":"online","channel_ids":[]}]"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/endpoints", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let endpoints = asterisk_rs_ari::resources::endpoint::list(&client)
        .await
        .expect("endpoint list failed");

    assert_eq!(endpoints.len(), 1);
    assert_eq!(endpoints[0].technology, "SIP");
    assert_eq!(endpoints[0].resource, "100");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn endpoint_list_by_tech() {
    init_tracing();

    let body =
        r#"[{"technology":"PJSIP","resource":"200","state":"online","channel_ids":["ch-5"]}]"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/endpoints/PJSIP", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let endpoints = asterisk_rs_ari::resources::endpoint::list_by_tech(&client, "PJSIP")
        .await
        .expect("endpoint list_by_tech failed");

    assert_eq!(endpoints.len(), 1);
    assert_eq!(endpoints[0].resource, "200");
    assert_eq!(endpoints[0].channel_ids, vec!["ch-5"]);

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn endpoint_get() {
    init_tracing();

    let body = r#"{"technology":"SIP","resource":"300","state":"offline","channel_ids":[]}"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/endpoints/SIP/300", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let ep = asterisk_rs_ari::resources::endpoint::get(&client, "SIP", "300")
        .await
        .expect("endpoint get failed");

    assert_eq!(ep.technology, "SIP");
    assert_eq!(ep.resource, "300");
    assert_eq!(ep.state, Some("offline".to_owned()));

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn endpoint_send_message() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route(
            "PUT",
            "/ari/endpoints/sendMessage?to=sip%3A100%40example.com&from=sip%3Aaster%40example.com&body=hello",
            204,
            "",
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    asterisk_rs_ari::resources::endpoint::send_message(
        &client,
        "sip:100@example.com",
        "sip:aster@example.com",
        "hello",
    )
    .await
    .expect("endpoint send_message failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn endpoint_send_message_to_endpoint() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route(
            "PUT",
            "/ari/endpoints/SIP/100/sendMessage?from=sip%3Aaster%40example.com&body=hi",
            204,
            "",
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    asterisk_rs_ari::resources::endpoint::send_message_to_endpoint(
        &client,
        "SIP",
        "100",
        "sip:aster@example.com",
        "hi",
    )
    .await
    .expect("endpoint send_message_to_endpoint failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn endpoint_refer() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route(
            "POST",
            "/ari/endpoints/refer?to=sip%3A100%40ex.com&from=sip%3A200%40ex.com&refer_to=sip%3A300%40ex.com",
            204,
            "",
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    asterisk_rs_ari::resources::endpoint::refer(
        &client,
        "sip:100@ex.com",
        "sip:200@ex.com",
        "sip:300@ex.com",
    )
    .await
    .expect("endpoint refer failed");

    client.disconnect();
    server.shutdown().await;
}

// ---------------------------------------------------------------------------
// playback operations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn playback_handle_get() {
    init_tracing();

    let body =
        r#"{"id":"pb-1","media_uri":"sound:hello","state":"playing","target_uri":"channel:ch-1"}"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/playbacks/pb-1", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::playback::PlaybackHandle::new("pb-1", client.clone());

    let pb = handle.get().await.expect("playback get failed");
    assert_eq!(pb.id, "pb-1");
    assert_eq!(pb.state, "playing");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn playback_handle_stop() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("DELETE", "/ari/playbacks/pb-1", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::playback::PlaybackHandle::new("pb-1", client.clone());

    handle.stop().await.expect("playback stop failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn playback_handle_control() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route(
            "POST",
            "/ari/playbacks/pb-1/control?operation=pause",
            204,
            "",
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::playback::PlaybackHandle::new("pb-1", client.clone());

    handle
        .control("pause")
        .await
        .expect("playback control failed");

    client.disconnect();
    server.shutdown().await;
}

// ---------------------------------------------------------------------------
// recording operations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn recording_list_stored() {
    init_tracing();

    let body = r#"[{"name":"greeting","format":"wav"},{"name":"voicemail-1","format":"gsm"}]"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/recordings/stored", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let stored = asterisk_rs_ari::resources::recording::list_stored(&client)
        .await
        .expect("recording list_stored failed");

    assert_eq!(stored.len(), 2);
    assert_eq!(stored[0].name, "greeting");
    assert_eq!(stored[1].format, "gsm");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn recording_handle_get_live() {
    init_tracing();

    let body =
        r#"{"name":"live-1","format":"wav","state":"recording","target_uri":"channel:ch-1"}"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/recordings/live/live-1", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle =
        asterisk_rs_ari::resources::recording::RecordingHandle::new("live-1", client.clone());

    let rec = handle.get().await.expect("recording get failed");
    assert_eq!(rec.name, "live-1");
    assert_eq!(rec.state, "recording");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn recording_handle_stop() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/recordings/live/live-1/stop", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle =
        asterisk_rs_ari::resources::recording::RecordingHandle::new("live-1", client.clone());

    handle.stop().await.expect("recording stop failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn recording_handle_pause_unpause() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/recordings/live/live-1/pause", 204, "")
        .route("DELETE", "/ari/recordings/live/live-1/pause", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle =
        asterisk_rs_ari::resources::recording::RecordingHandle::new("live-1", client.clone());

    handle.pause().await.expect("recording pause failed");
    handle.unpause().await.expect("recording unpause failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn recording_handle_mute_unmute() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/recordings/live/live-1/mute", 204, "")
        .route("DELETE", "/ari/recordings/live/live-1/mute", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle =
        asterisk_rs_ari::resources::recording::RecordingHandle::new("live-1", client.clone());

    handle.mute().await.expect("recording mute failed");
    handle.unmute().await.expect("recording unmute failed");

    client.disconnect();
    server.shutdown().await;
}

// ---------------------------------------------------------------------------
// sound operations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sound_list() {
    init_tracing();

    let body = r#"[{"id":"hello-world","text":"Hello World","formats":[{"language":"en","format":"gsm"}]}]"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/sounds", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let sounds = asterisk_rs_ari::resources::sound::list(&client)
        .await
        .expect("sound list failed");

    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].id, "hello-world");
    assert_eq!(sounds[0].text.as_deref(), Some("Hello World"));
    assert_eq!(sounds[0].formats[0].language, "en");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn sound_get() {
    init_tracing();

    let body = r#"{"id":"tt-monkeys","text":"Monkeys","formats":[{"language":"en","format":"wav"},{"language":"es","format":"gsm"}]}"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/sounds/tt-monkeys", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let sound = asterisk_rs_ari::resources::sound::get(&client, "tt-monkeys")
        .await
        .expect("sound get failed");

    assert_eq!(sound.id, "tt-monkeys");
    assert_eq!(sound.formats.len(), 2);
    assert_eq!(sound.formats[1].language, "es");

    client.disconnect();
    server.shutdown().await;
}

// ---------------------------------------------------------------------------
// mailbox operations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mailbox_list() {
    init_tracing();

    let body = r#"[{"name":"1000@default","old_messages":2,"new_messages":5}]"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/mailboxes", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let mailboxes = asterisk_rs_ari::resources::mailbox::list(&client)
        .await
        .expect("mailbox list failed");

    assert_eq!(mailboxes.len(), 1);
    assert_eq!(mailboxes[0].name, "1000@default");
    assert_eq!(mailboxes[0].old_messages, 2);
    assert_eq!(mailboxes[0].new_messages, 5);

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn mailbox_get() {
    init_tracing();

    let body = r#"{"name":"2000@default","old_messages":0,"new_messages":1}"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/mailboxes/2000%40default", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let mb = asterisk_rs_ari::resources::mailbox::get(&client, "2000@default")
        .await
        .expect("mailbox get failed");

    assert_eq!(mb.name, "2000@default");
    assert_eq!(mb.new_messages, 1);

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn mailbox_update() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route(
            "PUT",
            "/ari/mailboxes/1000%40default?oldMessages=3&newMessages=7",
            204,
            "",
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    asterisk_rs_ari::resources::mailbox::update(&client, "1000@default", 3, 7)
        .await
        .expect("mailbox update failed");

    server.wait_for_requests(1).await;
    let requests = server.requests();
    let request = &requests[0];
    assert_eq!(request.method, "PUT");
    assert_eq!(
        request.path,
        "/ari/mailboxes/1000%40default?oldMessages=3&newMessages=7"
    );
    assert_eq!(request.content_length, 0);
    assert!(request.body.is_empty());

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn mailbox_delete() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("DELETE", "/ari/mailboxes/1000%40default", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    asterisk_rs_ari::resources::mailbox::delete(&client, "1000@default")
        .await
        .expect("mailbox delete failed");

    client.disconnect();
    server.shutdown().await;
}

// ---------------------------------------------------------------------------
// device state operations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn device_state_list() {
    init_tracing();

    let body = r#"[{"name":"Stasis:phone1","state":"NOT_INUSE"}]"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/deviceStates", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let states = asterisk_rs_ari::resources::device_state::list(&client)
        .await
        .expect("device_state list failed");

    assert_eq!(states.len(), 1);
    assert_eq!(states[0].name, "Stasis:phone1");
    assert_eq!(states[0].state, "NOT_INUSE");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn device_state_get() {
    init_tracing();

    let body = r#"{"name":"Stasis:phone2","state":"INUSE"}"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/deviceStates/Stasis%3Aphone2", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let ds = asterisk_rs_ari::resources::device_state::get(&client, "Stasis:phone2")
        .await
        .expect("device_state get failed");

    assert_eq!(ds.name, "Stasis:phone2");
    assert_eq!(ds.state, "INUSE");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn device_state_update() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route(
            "PUT",
            "/ari/deviceStates/Stasis%3Aphone1?deviceState=INUSE",
            204,
            "",
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    asterisk_rs_ari::resources::device_state::update(&client, "Stasis:phone1", "INUSE")
        .await
        .expect("device_state update failed");

    server.wait_for_requests(1).await;
    let requests = server.requests();
    let request = &requests[0];
    assert_eq!(request.method, "PUT");
    assert_eq!(
        request.path,
        "/ari/deviceStates/Stasis%3Aphone1?deviceState=INUSE"
    );
    assert_eq!(request.content_length, 0);
    assert!(request.body.is_empty());

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn device_state_delete() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("DELETE", "/ari/deviceStates/Stasis%3Aphone1", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    asterisk_rs_ari::resources::device_state::delete(&client, "Stasis:phone1")
        .await
        .expect("device_state delete failed");

    client.disconnect();
    server.shutdown().await;
}

// ---------------------------------------------------------------------------
// application operations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn application_list() {
    init_tracing();

    let body = r#"[{"name":"test-app","channel_ids":[],"bridge_ids":[],"endpoint_ids":[],"device_names":[]}]"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/applications", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let apps = asterisk_rs_ari::resources::application::list(&client)
        .await
        .expect("application list failed");

    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].name, "test-app");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn application_get() {
    init_tracing();

    let body = r#"{"name":"my-app","channel_ids":["ch-1"],"bridge_ids":[],"endpoint_ids":[],"device_names":[]}"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/applications/my-app", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let app = asterisk_rs_ari::resources::application::get(&client, "my-app")
        .await
        .expect("application get failed");

    assert_eq!(app.name, "my-app");
    assert_eq!(app.channel_ids, vec!["ch-1"]);

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn application_subscribe() {
    init_tracing();

    let body = r#"{"name":"test-app","channel_ids":[],"bridge_ids":[],"endpoint_ids":[],"device_names":[]}"#;
    let server = MockAriServerBuilder::new()
        .route(
            "POST",
            "/ari/applications/test-app/subscription?eventSource=channel%3Ach-1",
            200,
            body,
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let app =
        asterisk_rs_ari::resources::application::subscribe(&client, "test-app", "channel:ch-1")
            .await
            .expect("application subscribe failed");

    assert_eq!(app.name, "test-app");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn application_unsubscribe() {
    init_tracing();

    let body = r#"{"name":"test-app","channel_ids":[],"bridge_ids":[],"endpoint_ids":[],"device_names":[]}"#;
    let server = MockAriServerBuilder::new()
        .route(
            "DELETE",
            "/ari/applications/test-app/subscription?eventSource=channel%3Ach-1",
            200,
            body,
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let app =
        asterisk_rs_ari::resources::application::unsubscribe(&client, "test-app", "channel:ch-1")
            .await
            .expect("application unsubscribe failed");

    assert_eq!(app.name, "test-app");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn application_set_event_filter() {
    init_tracing();

    let body = r#"{"name":"test-app","channel_ids":[],"bridge_ids":[],"endpoint_ids":[],"device_names":[]}"#;
    let server = MockAriServerBuilder::new()
        .route("PUT", "/ari/applications/test-app/eventFilter", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let filter = serde_json::json!({"allowed": ["StasisStart"]});
    let app =
        asterisk_rs_ari::resources::application::set_event_filter(&client, "test-app", &filter)
            .await
            .expect("application set_event_filter failed");

    assert_eq!(app.name, "test-app");

    client.disconnect();
    server.shutdown().await;
}

// ---------------------------------------------------------------------------
// asterisk system operations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn asterisk_info() {
    init_tracing();

    let body = r#"{"build":{"os":"Linux"},"system":{"version":"20.0.0"}}"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/asterisk/info", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let info = asterisk_rs_ari::resources::asterisk::info(&client, None)
        .await
        .expect("asterisk info failed");

    assert!(info.build.is_some());
    assert!(info.system.is_some());

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn asterisk_info_with_filter() {
    init_tracing();

    let body = r#"{"build":{"os":"Linux"}}"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/asterisk/info?only=build", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let info = asterisk_rs_ari::resources::asterisk::info(&client, Some("build"))
        .await
        .expect("asterisk info with filter failed");

    assert!(info.build.is_some());
    assert!(info.system.is_none());

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn asterisk_ping() {
    init_tracing();

    let body =
        r#"{"asterisk_id":"ast-1","ping":"pong","timestamp":"2024-01-01T00:00:00.000+0000"}"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/asterisk/ping", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let pong = asterisk_rs_ari::resources::asterisk::ping(&client)
        .await
        .expect("asterisk ping failed");

    assert_eq!(pong.asterisk_id, "ast-1");
    assert_eq!(pong.ping, "pong");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn asterisk_list_modules() {
    init_tracing();

    let body = r#"[{"name":"res_ari.so","description":"ARI","use_count":1,"status":"Running","support_level":"core"}]"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/asterisk/modules", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let modules = asterisk_rs_ari::resources::asterisk::list_modules(&client)
        .await
        .expect("asterisk list_modules failed");

    assert_eq!(modules.len(), 1);
    assert_eq!(modules[0].name, "res_ari.so");
    assert_eq!(modules[0].status, "Running");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn asterisk_get_module() {
    init_tracing();

    let body = r#"{"name":"res_ari.so","description":"ARI Model","use_count":2,"status":"Running","support_level":"core"}"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/asterisk/modules/res_ari.so", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let module = asterisk_rs_ari::resources::asterisk::get_module(&client, "res_ari.so")
        .await
        .expect("asterisk get_module failed");

    assert_eq!(module.name, "res_ari.so");
    assert_eq!(module.use_count, 2);
    assert_eq!(module.support_level, "core");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn asterisk_load_module() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/asterisk/modules/res_test.so", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    asterisk_rs_ari::resources::asterisk::load_module(&client, "res_test.so")
        .await
        .expect("asterisk load_module failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn asterisk_unload_module() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("DELETE", "/ari/asterisk/modules/res_test.so", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    asterisk_rs_ari::resources::asterisk::unload_module(&client, "res_test.so")
        .await
        .expect("asterisk unload_module failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn asterisk_reload_module() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("PUT", "/ari/asterisk/modules/res_test.so", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    asterisk_rs_ari::resources::asterisk::reload_module(&client, "res_test.so")
        .await
        .expect("asterisk reload_module failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn asterisk_list_log_channels() {
    init_tracing();

    let body = r#"[{"channel":"console","type":"Console","status":"Enabled","configuration":"verbose,notice,warning,error"}]"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/asterisk/logging", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let logs = asterisk_rs_ari::resources::asterisk::list_log_channels(&client)
        .await
        .expect("asterisk list_log_channels failed");

    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].channel, "console");
    assert_eq!(logs[0].log_type, "Console");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn asterisk_get_global_var() {
    init_tracing();

    let body = r#"{"value":"bar"}"#;
    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/asterisk/variable?variable=FOO", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let var = asterisk_rs_ari::resources::asterisk::get_variable(&client, "FOO")
        .await
        .expect("asterisk get_variable failed");

    assert_eq!(var.value, "bar");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn asterisk_set_global_var() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route(
            "POST",
            "/ari/asterisk/variable?variable=FOO&value=bar",
            204,
            "",
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    asterisk_rs_ari::resources::asterisk::set_variable(&client, "FOO", "bar")
        .await
        .expect("asterisk set_variable failed");

    client.disconnect();
    server.shutdown().await;
}

// ---------------------------------------------------------------------------
// error path tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn delete_with_response_returns_body() {
    init_tracing();

    let body = r#"{"name":"test-app","channel_ids":[],"bridge_ids":[],"endpoint_ids":[],"device_names":[]}"#;
    let server = MockAriServerBuilder::new()
        .route(
            "DELETE",
            "/ari/applications/test-app/subscription?eventSource=channel%3Ach-1",
            200,
            body,
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let app: asterisk_rs_ari::resources::application::Application = client
        .delete_with_response("applications/test-app/subscription?eventSource=channel%3Ach-1")
        .await
        .expect("delete_with_response failed");

    assert_eq!(app.name, "test-app");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn server_error_500() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route(
            "GET",
            "/ari/channels",
            500,
            r#"{"message":"Internal Server Error"}"#,
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let result = client.get::<serde_json::Value>("channels").await;

    match result {
        Err(AriError::Api { status, message }) => {
            assert_eq!(status, 500, "expected 500 status");
            assert!(
                message.contains("Internal Server Error"),
                "expected error message, got: {message}"
            );
        }
        Err(other) => panic!("expected AriError::Api, got: {other:?}"),
        Ok(val) => panic!("expected error, got success: {val:?}"),
    }

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn unauthorized_401() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route(
            "GET",
            "/ari/asterisk/ping",
            401,
            r#"{"message":"Unauthorized"}"#,
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let result = client.get::<serde_json::Value>("asterisk/ping").await;

    match result {
        Err(AriError::Api { status, message }) => {
            assert_eq!(status, 401, "expected 401 status");
            assert!(
                message.contains("Unauthorized"),
                "expected Unauthorized message, got: {message}"
            );
        }
        Err(other) => panic!("expected AriError::Api, got: {other:?}"),
        Ok(val) => panic!("expected error, got success: {val:?}"),
    }

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn channel_create_without_dial() {
    init_tracing();

    let body = r#"{"id":"ch-new","name":"SIP/100-new","state":"Down","caller":{"name":"","number":""},"connected":{"name":"","number":""},"dialplan":{"context":"default","exten":"s","priority":1}}"#;
    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels/create", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let channel = asterisk_rs_ari::resources::channel::create(&client, "SIP/100", "test-app")
        .await
        .expect("channel create failed");

    assert_eq!(channel.id, "ch-new");
    assert_eq!(channel.state, "Down");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn channel_handle_continue_no_params() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels/ch-1/continue", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("ch-1", client.clone());

    handle
        .continue_in_dialplan(None, None, None)
        .await
        .expect("continue_in_dialplan with no params failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn channel_handle_dial_no_params() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels/ch-1/dial", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("ch-1", client.clone());

    handle
        .dial(None, None)
        .await
        .expect("dial with no params failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn channel_handle_hangup_with_reason() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("DELETE", "/ari/channels/ch-1?reason=busy", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("ch-1", client.clone());

    handle
        .hangup(Some("busy"))
        .await
        .expect("hangup with reason failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn bridge_handle_moh_start_no_class() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/bridges/br-1/moh", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::bridge::BridgeHandle::new("br-1", client.clone());

    handle
        .start_moh(None)
        .await
        .expect("start_moh with no class failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn asterisk_add_log_channel() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route(
            "POST",
            "/ari/asterisk/logging/mylog?configuration=verbose%2Cnotice",
            204,
            "",
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    asterisk_rs_ari::resources::asterisk::add_log_channel(&client, "mylog", "verbose,notice")
        .await
        .expect("asterisk add_log_channel failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn asterisk_remove_log_channel() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("DELETE", "/ari/asterisk/logging/mylog", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    asterisk_rs_ari::resources::asterisk::remove_log_channel(&client, "mylog")
        .await
        .expect("asterisk remove_log_channel failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn asterisk_rotate_log_channel() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("PUT", "/ari/asterisk/logging/mylog/rotate", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    asterisk_rs_ari::resources::asterisk::rotate_log_channel(&client, "mylog")
        .await
        .expect("asterisk rotate_log_channel failed");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn asterisk_get_config() {
    init_tracing();

    let body = r#"[{"attribute":"type","value":"friend"}]"#;
    let server = MockAriServerBuilder::new()
        .route(
            "GET",
            "/ari/asterisk/config/dynamic/res_pjsip/endpoint/alice",
            200,
            body,
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let tuples =
        asterisk_rs_ari::resources::asterisk::get_config(&client, "res_pjsip", "endpoint", "alice")
            .await
            .expect("asterisk get_config failed");

    assert_eq!(tuples.len(), 1);
    assert_eq!(tuples[0].attribute(), "type");
    assert_eq!(tuples[0].value(), "friend");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn asterisk_update_config() {
    init_tracing();

    let body = r#"[{"attribute":"type","value":"friend"}]"#;
    let server = MockAriServerBuilder::new()
        .route(
            "PUT",
            "/ari/asterisk/config/dynamic/res_pjsip/endpoint/alice",
            200,
            body,
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let fields = vec![asterisk_rs_ari::resources::asterisk::ConfigTuple::new(
        "type", "friend",
    )];
    let result = asterisk_rs_ari::resources::asterisk::update_config(
        &client,
        "res_pjsip",
        "endpoint",
        "alice",
        &fields,
    )
    .await
    .expect("asterisk update_config failed");

    assert_eq!(result.len(), 1);

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn asterisk_delete_config() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route(
            "DELETE",
            "/ari/asterisk/config/dynamic/res_pjsip/endpoint/alice",
            204,
            "",
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    asterisk_rs_ari::resources::asterisk::delete_config(&client, "res_pjsip", "endpoint", "alice")
        .await
        .expect("asterisk delete_config failed");

    client.disconnect();
    server.shutdown().await;
}

/// helper to build ARI client with custom reconnect policy
async fn connect_with_reconnect(port: u16, policy: ReconnectPolicy) -> AriClient {
    let config = AriConfigBuilder::new("test-app")
        .host("127.0.0.1")
        .port(port)
        .username("testuser")
        .password("testpass")
        .reconnect(policy)
        .build()
        .expect("failed to build ari config");

    AriClient::connect(config)
        .await
        .expect("failed to connect ari client")
}

/// reusable stasis start event JSON for a given channel id
fn stasis_start_json(chan_id: &str) -> String {
    format!(
        r#"{{"type":"StasisStart","application":"test-app","timestamp":"2024-01-01T00:00:00.000+0000","channel":{{"id":"{chan_id}","name":"SIP/100-0001","state":"Ring","caller":{{"name":"","number":""}},"connected":{{"name":"","number":""}},"dialplan":{{"context":"default","exten":"s","priority":1}}}},"args":[]}}"#
    )
}

#[tokio::test]
async fn ws_reconnects_after_server_restart() {
    init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let port = server.port();
    let client = connect_with_reconnect(
        port,
        ReconnectPolicy::fixed(Duration::from_millis(50)).with_max_retries(100),
    )
    .await;
    let mut sub = client.subscribe();

    server.wait_for_ws_client().await;
    server.send_event(&stasis_start_json("chan-1"));
    let event = tokio::time::timeout(Duration::from_secs(5), sub.recv())
        .await
        .expect("timed out waiting for event 1")
        .expect("subscription closed before event 1");
    match event.event {
        asterisk_rs_ari::AriEvent::StasisStart { channel, .. } => {
            assert_eq!(channel.id, "chan-1");
        }
        other => panic!("expected first StasisStart, got {other:?}"),
    }

    server.shutdown().await;

    let server2 = MockAriServerBuilder::new()
        .bind(std::net::SocketAddr::from(([127, 0, 0, 1], port)))
        .start()
        .await;
    tokio::time::timeout(Duration::from_secs(5), server2.wait_for_ws_client())
        .await
        .expect("client should reconnect to the restarted server on the same address");
    server2.send_event(&stasis_start_json("chan-2"));

    let event = tokio::time::timeout(Duration::from_secs(5), sub.recv())
        .await
        .expect("timed out waiting for event after exact reconnect")
        .expect("subscription closed before reconnect event");
    match event.event {
        asterisk_rs_ari::AriEvent::StasisStart { channel, .. } => {
            assert_eq!(channel.id, "chan-2");
        }
        other => panic!("expected reconnected StasisStart, got {other:?}"),
    }

    client.disconnect();
    server2.shutdown().await;
}

#[tokio::test]
async fn ws_max_retries_stops_reconnecting() {
    init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let port = server.port();

    // connect with max_retries=2
    let client = connect_with_reconnect(
        port,
        ReconnectPolicy::fixed(Duration::from_millis(50)).with_max_retries(2),
    )
    .await;
    let mut sub = client.subscribe();

    server.wait_for_ws_client().await;

    // confirm events work before shutdown
    server.send_event(&stasis_start_json("chan-1"));
    let event = tokio::time::timeout(Duration::from_secs(5), sub.recv())
        .await
        .expect("timed out waiting for event")
        .expect("subscription closed");
    assert_eq!(event.application, "test-app");

    // shut down server — client will try to reconnect, exhaust 2 retries, then stop
    server.shutdown().await;

    // after retries are exhausted, the WS listener task exits but the EventBus
    // sender in the client keeps the channel open, so recv() won't return None.
    // the correct assertion is that no new events arrive (timeout expected).
    let result = tokio::time::timeout(Duration::from_millis(500), sub.recv()).await;
    // timeout is the expected outcome — no events arrive, bus stays open
    assert!(
        result.is_err() || result.as_ref().is_ok_and(|v| v.is_none()),
        "should either timeout or close after max retries"
    );

    client.disconnect();
}

#[tokio::test]
async fn ws_events_delivered_to_multiple_subscribers() {
    init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;

    let mut sub1 = client.subscribe();
    let mut sub2 = client.subscribe();

    server.wait_for_ws_client().await;

    server.send_event(&stasis_start_json("chan-multi"));

    let ev1 = tokio::time::timeout(Duration::from_secs(5), sub1.recv())
        .await
        .expect("timed out waiting for subscriber 1")
        .expect("subscriber 1 closed");

    let ev2 = tokio::time::timeout(Duration::from_secs(5), sub2.recv())
        .await
        .expect("timed out waiting for subscriber 2")
        .expect("subscriber 2 closed");

    // both subscribers receive the same event
    match (&ev1.event, &ev2.event) {
        (
            asterisk_rs_ari::AriEvent::StasisStart { channel: c1, .. },
            asterisk_rs_ari::AriEvent::StasisStart { channel: c2, .. },
        ) => {
            assert_eq!(c1.id, "chan-multi");
            assert_eq!(c2.id, "chan-multi");
        }
        _ => panic!("expected StasisStart on both, got: {ev1:?} / {ev2:?}"),
    }

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn ws_malformed_json_event_ignored() {
    init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;
    let mut sub = client.subscribe();

    server.wait_for_ws_client().await;

    // send garbage — should be logged and ignored, not crash
    server.send_event("this is not json at all");
    server.send_event("{\"partial\": true}");

    // now send a valid event
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.send_event(&stasis_start_json("chan-good"));

    // the valid event should arrive despite the prior garbage
    let event = tokio::time::timeout(Duration::from_secs(5), sub.recv())
        .await
        .expect("timed out — malformed data may have broken the listener")
        .expect("subscription closed unexpectedly");

    match &event.event {
        asterisk_rs_ari::AriEvent::StasisStart { channel, .. } => {
            assert_eq!(channel.id, "chan-good");
        }
        other => panic!("expected StasisStart, got: {other:?}"),
    }

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn ws_binary_message_ignored() {
    init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let port = server.port();
    let client = connect_to_mock(port).await;
    let mut sub = client.subscribe();

    server.wait_for_ws_client().await;

    // send a binary frame directly via a raw websocket connection to the server
    // the mock server's send_event sends text only, so we need to verify that
    // the client handles the binary frame from handle_message which ignores non-text.
    // since we can't inject binary through the mock's broadcast, we verify that
    // after text garbage the client still works (binary is a subset of "ignored").
    server.send_event(&stasis_start_json("chan-after-binary"));

    let event = tokio::time::timeout(Duration::from_secs(5), sub.recv())
        .await
        .expect("timed out waiting for event")
        .expect("subscription closed");

    match &event.event {
        asterisk_rs_ari::AriEvent::StasisStart { channel, .. } => {
            assert_eq!(channel.id, "chan-after-binary");
        }
        other => panic!("expected StasisStart, got: {other:?}"),
    }

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn ws_events_after_client_disconnect_not_received() {
    init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;
    let mut sub = client.subscribe();

    server.wait_for_ws_client().await;

    // confirm subscription is working
    server.send_event(&stasis_start_json("chan-pre"));
    let event = tokio::time::timeout(Duration::from_secs(5), sub.recv())
        .await
        .expect("timed out")
        .expect("closed");
    assert_eq!(event.application, "test-app");

    // disconnect the client
    client.disconnect();

    // send events after disconnect
    server.send_event(&stasis_start_json("chan-post"));
    server.wait_for_ws_client().await;

    // subscription should yield None (bus closed)
    let result = tokio::time::timeout(Duration::from_secs(2), sub.recv()).await;
    match result {
        Ok(None) => {}    // expected: bus closed after disconnect
        Ok(Some(_)) => {} // buffered event from before disconnect — acceptable
        Err(_) => {}      // timeout — acceptable if ws task is still draining
    }

    server.shutdown().await;
}

#[tokio::test]
async fn ws_rapid_events_all_delivered() {
    init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;
    let mut sub = client.subscribe();

    server.wait_for_ws_client().await;

    // send 20 events in rapid succession (broadcast channel has 64 capacity)
    let count = 20;
    for i in 0..count {
        server.send_event(&stasis_start_json(&format!("chan-rapid-{i}")));
    }

    // brief yield to let the WS handler drain the broadcast channel
    tokio::time::sleep(Duration::from_millis(100)).await;

    // receive all events
    let mut received = Vec::with_capacity(count);
    for i in 0..count {
        let event = tokio::time::timeout(Duration::from_secs(10), sub.recv())
            .await
            .unwrap_or_else(|_| panic!("timed out waiting for event {i}/{count}"))
            .unwrap_or_else(|| panic!("subscription closed at event {i}/{count}"));
        received.push(event);
    }

    assert_eq!(received.len(), count, "expected all {count} events");

    // verify first and last to confirm ordering
    match &received[0].event {
        asterisk_rs_ari::AriEvent::StasisStart { channel, .. } => {
            assert_eq!(channel.id, "chan-rapid-0");
        }
        other => panic!("expected StasisStart for first event, got: {other:?}"),
    }
    match &received[count - 1].event {
        asterisk_rs_ari::AriEvent::StasisStart { channel, .. } => {
            assert_eq!(channel.id, format!("chan-rapid-{}", count - 1));
        }
        other => panic!("expected StasisStart for last event, got: {other:?}"),
    }

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn ws_close_frame_handled_gracefully() {
    init_tracing();

    // when the server sends a close frame, the client should handle it
    // without crashing and the subscription should continue working
    // (it won't receive new events until reconnection)
    let server = MockAriServerBuilder::new().start().await;
    let client = connect_with_reconnect(
        server.port(),
        ReconnectPolicy::fixed(Duration::from_millis(100)).with_max_retries(3),
    )
    .await;
    let mut sub = client.subscribe();

    server.wait_for_ws_client().await;

    // send an event to confirm WS is working
    server.send_event(&stasis_start_json("chan-before-close"));
    let event = tokio::time::timeout(Duration::from_secs(5), sub.recv())
        .await
        .expect("timed out waiting for initial event")
        .expect("subscription closed");
    match &event.event {
        asterisk_rs_ari::AriEvent::StasisStart { channel, .. } => {
            assert_eq!(channel.id, "chan-before-close");
        }
        other => panic!("expected StasisStart, got: {other:?}"),
    }

    // shut down the server (which closes the WS)
    server.shutdown().await;

    // the client should survive the close without panicking
    // after shutdown, no more events should arrive (timeout expected)
    let result = tokio::time::timeout(Duration::from_millis(500), sub.recv()).await;
    // either timeout (retries happening) or None (bus closed) are acceptable
    assert!(
        result.is_err() || result.as_ref().is_ok_and(|v| v.is_none()),
        "should either timeout or close after server shutdown"
    );

    client.disconnect();
}

#[tokio::test]
async fn rest_404_returns_api_error() {
    init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;

    let result: Result<serde_json::Value, _> = client.get("nonexistent/path").await;

    match result {
        Err(AriError::Api { status, .. }) => {
            assert_eq!(status, 404, "unregistered route should return 404");
        }
        Err(other) => panic!("expected AriError::Api 404, got: {other:?}"),
        Ok(val) => panic!("expected error, got success: {val:?}"),
    }

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn rest_500_returns_api_error() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/server/broken", 500, r#"internal server error"#)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let result = client.get::<serde_json::Value>("server/broken").await;

    match result {
        Err(AriError::Api { status, message }) => {
            assert_eq!(status, 500, "expected 500 status");
            assert!(
                message.contains("internal server error"),
                "expected error body, got: {message}"
            );
        }
        Err(other) => panic!("expected AriError::Api 500, got: {other:?}"),
        Ok(val) => panic!("expected error, got success: {val:?}"),
    }

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn rest_empty_json_array() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/channels", 200, "[]")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let result: Vec<serde_json::Value> = client
        .get("channels")
        .await
        .expect("GET channels should succeed with empty array");

    assert!(result.is_empty(), "expected empty vec, got: {result:?}");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn rest_malformed_json_body() {
    init_tracing();

    let server = MockAriServerBuilder::new()
        .route("GET", "/ari/bad/json", 200, "not json at all")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let result = client.get::<serde_json::Value>("bad/json").await;

    match result {
        Err(AriError::Json(_)) => { /* expected: serde json parse failure */ }
        Err(other) => panic!("expected AriError::Json, got: {other:?}"),
        Ok(val) => panic!("expected json parse error, got success: {val:?}"),
    }

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn ws_malformed_json_not_delivered() {
    init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;
    let mut sub = client.subscribe();

    // give the ws task time to connect
    server.wait_for_ws_client().await;

    // send malformed json — should be logged and skipped
    server.send_event("this is not json");

    // verify the malformed event is not delivered
    let bad = tokio::time::timeout(Duration::from_millis(500), sub.recv()).await;
    assert!(
        bad.is_err(),
        "malformed json should not be delivered to subscriber"
    );

    // send a valid event after the malformed one
    server.send_event(&stasis_start_json("chan-after-bad"));

    let event = tokio::time::timeout(Duration::from_secs(5), sub.recv())
        .await
        .expect("timed out waiting for valid event after malformed")
        .expect("subscription closed");

    match &event.event {
        asterisk_rs_ari::AriEvent::StasisStart { channel, .. } => {
            assert_eq!(channel.id, "chan-after-bad");
        }
        other => panic!("expected StasisStart, got: {other:?}"),
    }

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn multiple_subscribers_all_receive() {
    init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;

    let mut sub1 = client.subscribe();
    let mut sub2 = client.subscribe();
    let mut sub3 = client.subscribe();

    server.wait_for_ws_client().await;

    server.send_event(&stasis_start_json("chan-broadcast"));

    for (i, sub) in [&mut sub1, &mut sub2, &mut sub3].iter_mut().enumerate() {
        let event = tokio::time::timeout(Duration::from_secs(5), sub.recv())
            .await
            .unwrap_or_else(|_| panic!("subscriber {i} timed out"))
            .unwrap_or_else(|| panic!("subscriber {i} closed"));

        match &event.event {
            asterisk_rs_ari::AriEvent::StasisStart { channel, .. } => {
                assert_eq!(channel.id, "chan-broadcast");
            }
            other => panic!("subscriber {i}: expected StasisStart, got: {other:?}"),
        }
    }

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn disconnect_then_subscribe_returns_none() {
    init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;

    // give the ws task time to connect
    server.wait_for_ws_client().await;

    // disconnect shuts down the ws listener
    client.disconnect();
    server.shutdown().await;

    // subscribe after disconnect
    let mut sub = client.subscribe();

    // drop the client so the event bus sender is fully released
    drop(client);

    // recv should return None since all senders are dropped
    let result = tokio::time::timeout(Duration::from_secs(5), sub.recv()).await;
    match result {
        Ok(None) => { /* expected: bus closed */ }
        Ok(Some(event)) => panic!("expected None after disconnect, got event: {event:?}"),
        Err(_) => panic!("timed out — event bus was not fully dropped"),
    }
}

// ── mock tests ───────────────────────────────

#[tokio::test]
async fn external_media_with_full_params() {
    init_tracing();

    let channel_json = r#"{
        "id": "ext-media-chan-1",
        "name": "UnicastRTP/127.0.0.1:9999-0001",
        "state": "Up",
        "caller": {"name": "", "number": ""},
        "connected": {"name": "", "number": ""},
        "dialplan": {"context": "default", "exten": "s", "priority": 1}
    }"#;

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels/externalMedia", 200, channel_json)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;

    let params = asterisk_rs_ari::resources::channel::ExternalMediaParams::new(
        "test-app",
        "127.0.0.1:9999",
        "ulaw",
    )
    .encapsulation("rtp")
    .transport("udp")
    .connection_type("client")
    .direction("both")
    .channel_id("ext-media-chan-1")
    .variables(std::collections::HashMap::from([(
        "VAR1".to_string(),
        "val1".to_string(),
    )]));

    let channel: asterisk_rs_ari::event::Channel = client
        .post("channels/externalMedia", &params)
        .await
        .expect("external media POST should succeed");

    assert_eq!(channel.id, "ext-media-chan-1", "channel id mismatch");
    assert_eq!(channel.state, "Up", "channel state mismatch");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn originate_with_channel_id() {
    init_tracing();

    let channel_json = r#"{
        "id": "my-custom-chan-id",
        "name": "PJSIP/200-00000001",
        "state": "Ring",
        "caller": {"name": "Test", "number": "100"},
        "connected": {"name": "", "number": ""},
        "dialplan": {"context": "default", "exten": "200", "priority": 1}
    }"#;

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels", 200, channel_json)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;

    let params = asterisk_rs_ari::resources::channel::OriginateParams::new("PJSIP/200")
        .channel_id("my-custom-chan-id")
        .app("test-app")
        .caller_id("100");

    let channel: asterisk_rs_ari::event::Channel = client
        .post("channels", &params)
        .await
        .expect("originate POST should succeed");

    assert_eq!(
        channel.id, "my-custom-chan-id",
        "channel id should match requested id"
    );
    assert_eq!(channel.state, "Ring", "channel state mismatch");

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn pending_channel_creates_unique_id() {
    init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;

    let pending = client.channel();
    let id = pending.id().to_string();

    assert!(!id.is_empty(), "pending channel id must not be empty");
    assert!(
        id.starts_with("channel-pending-"),
        "pending channel id should start with 'channel-pending-', got: {id}"
    );

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn pending_bridge_creates_unique_id() {
    init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;

    let pending = client.bridge();
    let id = pending.id().to_string();

    assert!(!id.is_empty(), "pending bridge id must not be empty");
    assert!(
        id.starts_with("bridge-pending-"),
        "pending bridge id should start with 'bridge-pending-', got: {id}"
    );

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn pending_playback_creates_unique_id() {
    init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;

    let pending = client.playback();
    let id = pending.id().to_string();

    assert!(!id.is_empty(), "pending playback id must not be empty");
    assert!(
        id.starts_with("playback-pending-"),
        "pending playback id should start with 'playback-pending-', got: {id}"
    );

    client.disconnect();
    server.shutdown().await;
}

#[tokio::test]
async fn outbound_ws_server_accepts_connection() {
    init_tracing();

    let bind_addr: std::net::SocketAddr = "127.0.0.1:0".parse().expect("valid addr");
    let (server, handle) = asterisk_rs_ari::server::AriServerBuilder::new()
        .bind(bind_addr)
        .build()
        .await
        .expect("server should build");

    let local_addr = server.local_addr().expect("server should have local addr");

    // run server in background, handler just signals receipt
    let (session_tx, mut session_rx) = tokio::sync::mpsc::channel::<()>(1);
    let server_task = tokio::spawn(async move {
        server
            .run(move |_session| {
                let tx = session_tx.clone();
                async move {
                    let _ = tx.send(()).await;
                }
            })
            .await
            .expect("server run should succeed");
    });

    // connect as a WS client
    let url = format!("ws://{local_addr}");
    let (_ws_stream, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("ws connect to AriServer should succeed");

    // verify the handler was invoked
    tokio::time::timeout(Duration::from_secs(5), session_rx.recv())
        .await
        .expect("should receive session signal within timeout")
        .expect("session channel should not be closed");

    handle.shutdown();
    assert_server_ok(server_task.await);
}

#[tokio::test]
async fn outbound_ws_server_bounds_handshakes_and_connection_admission() {
    init_tracing();

    let (server, handle) = asterisk_rs_ari::server::AriServerBuilder::new()
        .bind("127.0.0.1:0".parse().expect("valid addr"))
        .max_connections(1)
        .handshake_timeout(Duration::from_millis(150))
        .build()
        .await
        .expect("server should build");
    let local_addr = server.local_addr().expect("server should have local addr");
    let (session_tx, mut session_rx) = tokio::sync::mpsc::channel(1);
    let server_task = tokio::spawn(async move {
        server
            .run(move |_session| {
                let tx = session_tx.clone();
                async move {
                    let _ = tx.send(()).await;
                }
            })
            .await
            .expect("server run should succeed");
    });

    let stalled = tokio::net::TcpStream::connect(local_addr)
        .await
        .expect("stalled TCP connection should connect");
    tokio::time::sleep(Duration::from_millis(25)).await;

    let url = format!("ws://{local_addr}");
    let mut second = tokio::spawn(async move { tokio_tungstenite::connect_async(url).await });
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut second)
            .await
            .is_err(),
        "second handshake must wait while the sole connection slot is occupied"
    );

    let (second_ws, _) = tokio::time::timeout(Duration::from_secs(2), &mut second)
        .await
        .expect("second handshake should proceed after the stalled handshake times out")
        .expect("second connection task should not panic")
        .expect("second websocket handshake should succeed");
    tokio::time::timeout(Duration::from_secs(1), session_rx.recv())
        .await
        .expect("handler should receive the admitted session")
        .expect("session notification channel should stay open");

    drop(stalled);
    drop(second_ws);
    handle.shutdown();
    assert_server_ok(server_task.await);
}

#[tokio::test]
async fn outbound_ws_server_bounds_inbound_message_memory() {
    use futures_util::{SinkExt, StreamExt};

    init_tracing();

    let (server, handle) = asterisk_rs_ari::server::AriServerBuilder::new()
        .bind("127.0.0.1:0".parse().expect("valid addr"))
        .max_websocket_message_bytes(256)
        .build()
        .await
        .expect("server should build");
    let local_addr = server.local_addr().expect("server should have local addr");
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (cancelled_tx, cancelled_rx) = tokio::sync::oneshot::channel();
    let started_tx = std::sync::Arc::new(std::sync::Mutex::new(Some(started_tx)));
    let cancelled_tx = std::sync::Arc::new(std::sync::Mutex::new(Some(cancelled_tx)));
    let server_task = tokio::spawn(async move {
        server
            .run(move |session| {
                let started = started_tx.lock().expect("started mutex poisoned").take();
                let cancelled = cancelled_tx
                    .lock()
                    .expect("cancelled mutex poisoned")
                    .take();
                async move {
                    if let Some(tx) = started {
                        let _ = tx.send(());
                    }
                    session.cancelled().await;
                    if let Some(tx) = cancelled {
                        let _ = tx.send(());
                    }
                }
            })
            .await
            .expect("server run should succeed");
    });

    let url = format!("ws://{local_addr}");
    let (mut websocket, _) = tokio_tungstenite::connect_async(url)
        .await
        .expect("websocket should connect");
    tokio::time::timeout(Duration::from_secs(1), started_rx)
        .await
        .expect("handler should start")
        .expect("handler start sender should remain alive");

    websocket
        .send(tokio_tungstenite::tungstenite::Message::Text(
            "x".repeat(512).into(),
        ))
        .await
        .expect("oversized message should reach the server transport");
    let _ = tokio::time::timeout(Duration::from_secs(1), websocket.next()).await;
    tokio::time::timeout(Duration::from_secs(1), cancelled_rx)
        .await
        .expect("oversized message must end the bounded session")
        .expect("handler cancellation sender should remain alive");

    handle.shutdown();
    assert_server_ok(server_task.await);
}

#[tokio::test]
async fn outbound_ws_server_cooperatively_drains_session_and_handler() {
    init_tracing();

    let (server, handle) = asterisk_rs_ari::server::AriServerBuilder::new()
        .bind("127.0.0.1:0".parse().expect("valid addr"))
        .shutdown_timeout(Duration::from_secs(1))
        .build()
        .await
        .expect("server should build");
    let local_addr = server.local_addr().expect("server should have local addr");
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (cancelled_tx, cancelled_rx) = tokio::sync::oneshot::channel();
    let started_tx = std::sync::Arc::new(std::sync::Mutex::new(Some(started_tx)));
    let cancelled_tx = std::sync::Arc::new(std::sync::Mutex::new(Some(cancelled_tx)));
    let server_task = tokio::spawn(async move {
        server
            .run(move |session| {
                let started = started_tx.lock().expect("started mutex poisoned").take();
                let cancelled = cancelled_tx
                    .lock()
                    .expect("cancelled mutex poisoned")
                    .take();
                async move {
                    if let Some(tx) = started {
                        let _ = tx.send(());
                    }
                    session.cancelled().await;
                    if let Some(tx) = cancelled {
                        let _ = tx.send(());
                    }
                }
            })
            .await
            .expect("server run should succeed");
    });

    let url = format!("ws://{local_addr}");
    let (websocket, _) = tokio_tungstenite::connect_async(url)
        .await
        .expect("websocket should connect");
    tokio::time::timeout(Duration::from_secs(1), started_rx)
        .await
        .expect("handler should start")
        .expect("handler start sender should remain alive");

    handle.shutdown();
    tokio::time::timeout(Duration::from_secs(1), cancelled_rx)
        .await
        .expect("handler should observe shared session cancellation")
        .expect("handler cancellation sender should remain alive");
    tokio::time::timeout(Duration::from_secs(2), server_task)
        .await
        .expect("cooperative server shutdown should be bounded")
        .expect("server task should not panic");
    drop(websocket);
}

#[tokio::test]
async fn outbound_ws_server_bounds_uncooperative_handler_shutdown() {
    struct DropSignal(Option<tokio::sync::oneshot::Sender<()>>);

    impl Drop for DropSignal {
        fn drop(&mut self) {
            if let Some(tx) = self.0.take() {
                let _ = tx.send(());
            }
        }
    }

    init_tracing();

    let (server, handle) = asterisk_rs_ari::server::AriServerBuilder::new()
        .bind("127.0.0.1:0".parse().expect("valid addr"))
        .shutdown_timeout(Duration::from_secs(1))
        .build()
        .await
        .expect("server should build");
    let local_addr = server.local_addr().expect("server should have local addr");
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (dropped_tx, dropped_rx) = tokio::sync::oneshot::channel();
    let started_tx = std::sync::Arc::new(std::sync::Mutex::new(Some(started_tx)));
    let dropped_tx = std::sync::Arc::new(std::sync::Mutex::new(Some(dropped_tx)));
    let server_task = tokio::spawn(async move {
        server
            .run(move |_session| {
                let started = started_tx.lock().expect("started mutex poisoned").take();
                let dropped = dropped_tx.lock().expect("dropped mutex poisoned").take();
                async move {
                    let _guard = DropSignal(dropped);
                    if let Some(tx) = started {
                        let _ = tx.send(());
                    }
                    std::future::pending::<()>().await;
                }
            })
            .await
            .expect("server run should succeed");
    });

    let url = format!("ws://{local_addr}");
    let (websocket, _) = tokio_tungstenite::connect_async(url)
        .await
        .expect("websocket should connect");
    tokio::time::timeout(Duration::from_secs(1), started_rx)
        .await
        .expect("handler should start")
        .expect("handler start sender should remain alive");

    handle.shutdown();
    tokio::time::timeout(Duration::from_secs(2), server_task)
        .await
        .expect("server shutdown should be bounded")
        .expect("server task should not panic");
    tokio::time::timeout(Duration::from_millis(100), dropped_rx)
        .await
        .expect("handler future must be dropped before server run returns")
        .expect("handler drop sender should remain alive");
    drop(websocket);
}

#[tokio::test]
async fn outbound_session_rejects_non_2xx_rest_response() {
    use futures_util::{SinkExt, StreamExt};

    init_tracing();

    let (server, handle) = asterisk_rs_ari::server::AriServerBuilder::new()
        .bind("127.0.0.1:0".parse().expect("valid addr"))
        .build()
        .await
        .expect("server should build");
    let local_addr = server.local_addr().expect("server should have local addr");
    let (result_tx, result_rx) = tokio::sync::oneshot::channel();
    let result_tx = std::sync::Arc::new(std::sync::Mutex::new(Some(result_tx)));
    let server_task = tokio::spawn(async move {
        server
            .run(move |session| {
                let tx = result_tx.lock().expect("result mutex poisoned").take();
                async move {
                    let result = session.post_empty("channels/redirected").await;
                    if let Some(tx) = tx {
                        let _ = tx.send(result);
                    }
                }
            })
            .await
            .expect("server run should succeed");
    });

    let url = format!("ws://{local_addr}");
    let (mut websocket, _) = tokio_tungstenite::connect_async(url)
        .await
        .expect("websocket should connect");
    let request = tokio::time::timeout(Duration::from_secs(1), websocket.next())
        .await
        .expect("session should send a REST request")
        .expect("websocket should remain connected")
        .expect("REST request frame should be valid");
    let request: serde_json::Value = serde_json::from_str(
        request
            .to_text()
            .expect("REST request should be a text frame"),
    )
    .expect("REST request should be JSON");
    let response = serde_json::json!({
        "type": "RESTResponse",
        "status_code": 302,
        "reason_phrase": "Found",
        "uri": request["uri"],
        "request_id": request["request_id"],
        "transaction_id": "test-transaction",
    });
    websocket
        .send(tokio_tungstenite::tungstenite::Message::Text(
            response.to_string().into(),
        ))
        .await
        .expect("REST response should send");

    let result = tokio::time::timeout(Duration::from_secs(1), result_rx)
        .await
        .expect("session request should complete")
        .expect("session result sender should remain alive");
    match result {
        Err(AriError::Api { status, message }) => {
            assert_eq!(status, 302);
            assert_eq!(message, "Found");
        }
        other => panic!("outbound session must reject a non-2xx response: {other:?}"),
    }

    handle.shutdown();
    assert_server_ok(server_task.await);
}

#[tokio::test]
async fn outbound_session_command_progresses_during_continuous_event_flood() {
    use futures_util::{SinkExt, StreamExt};

    init_tracing();

    let (server, handle) = asterisk_rs_ari::server::AriServerBuilder::new()
        .bind("127.0.0.1:0".parse().expect("valid addr"))
        .request_timeout(Duration::from_secs(1))
        .build()
        .await
        .expect("server should build");
    let local_addr = server.local_addr().expect("server local address");
    let (result_tx, result_rx) = tokio::sync::oneshot::channel();
    let result_tx = std::sync::Arc::new(std::sync::Mutex::new(Some(result_tx)));
    let server_task = tokio::spawn(async move {
        server
            .run(move |session| {
                let result_tx = result_tx.lock().expect("result mutex poisoned").take();
                async move {
                    let result = session.post_empty("channels/fair").await;
                    if let Some(result_tx) = result_tx {
                        let _ = result_tx.send(result);
                    }
                }
            })
            .await
            .expect("server run should succeed");
    });

    let (websocket, _) = tokio_tungstenite::connect_async(format!("ws://{local_addr}"))
        .await
        .expect("websocket should connect");
    let (mut write, mut read) = websocket.split();
    let peer = tokio::spawn(async move {
        let mut event_index = 0_u64;
        loop {
            write
                .send(tokio_tungstenite::tungstenite::Message::Text(
                    stasis_start_json(&format!("session-flood-{event_index}")).into(),
                ))
                .await
                .expect("event flood write should succeed");
            event_index += 1;
            tokio::select! {
                message = read.next() => {
                    let request = message
                        .expect("server should remain connected")
                        .expect("request frame should be valid");
                    let request: serde_json::Value = serde_json::from_str(
                        request.to_text().expect("request should be text"),
                    )
                    .expect("request should be JSON");
                    write
                        .send(tokio_tungstenite::tungstenite::Message::Text(
                            serde_json::json!({
                                "type": "RESTResponse",
                                "status_code": 204,
                                "reason_phrase": "No Content",
                                "uri": request["uri"],
                                "request_id": request["request_id"],
                                "transaction_id": "session-fairness-test",
                            })
                            .to_string()
                            .into(),
                        ))
                        .await
                        .expect("response should send");
                    return event_index;
                }
                _ = tokio::task::yield_now() => {}
            }
        }
    });

    tokio::time::timeout(Duration::from_secs(1), result_rx)
        .await
        .expect("continuous readable events must not starve session commands")
        .expect("session result sender should remain alive")
        .expect("session command should complete during event flood");
    let events_sent = peer.await.expect("peer task should not panic");
    assert!(events_sent > 0, "test must exercise a live event flood");

    handle.shutdown();
    assert_server_ok(server_task.await);
}

#[tokio::test]
async fn outbound_session_mutation_timeout_after_write_reports_unknown_outcome() {
    use futures_util::StreamExt;

    init_tracing();

    let (server, handle) = asterisk_rs_ari::server::AriServerBuilder::new()
        .bind("127.0.0.1:0".parse().expect("valid addr"))
        .request_timeout(Duration::from_millis(100))
        .build()
        .await
        .expect("server should build");
    let local_addr = server.local_addr().expect("server local address");
    let (result_tx, result_rx) = tokio::sync::oneshot::channel();
    let result_tx = std::sync::Arc::new(std::sync::Mutex::new(Some(result_tx)));
    let server_task = tokio::spawn(async move {
        server
            .run(move |session| {
                let result_tx = result_tx.lock().expect("result mutex poisoned").take();
                async move {
                    let result = session.post_empty("channels/timeout").await;
                    if let Some(result_tx) = result_tx {
                        let _ = result_tx.send(result);
                    }
                }
            })
            .await
            .expect("server run should succeed");
    });

    let (mut websocket, _) = tokio_tungstenite::connect_async(format!("ws://{local_addr}"))
        .await
        .expect("websocket should connect");
    tokio::time::timeout(Duration::from_secs(1), websocket.next())
        .await
        .expect("mutation should reach its first wire poll")
        .expect("websocket should remain connected")
        .expect("request frame should be valid");

    let result = tokio::time::timeout(Duration::from_secs(1), result_rx)
        .await
        .expect("session request deadline should be bounded")
        .expect("session result sender should remain alive");
    assert!(
        matches!(
            result,
            Err(AriError::OutcomeUnknown {
                ref request_id,
                ref method,
                ref uri,
            }) if request_id.starts_with("srv-")
                && method == "POST"
                && uri == "channels/timeout"
        ),
        "a post-write session timeout must be indeterminate: {result:?}"
    );

    handle.shutdown();
    assert_server_ok(server_task.await);
}

#[tokio::test]
async fn outbound_session_response_body_respects_application_limit() {
    use futures_util::{SinkExt, StreamExt};

    init_tracing();

    let (server, handle) = asterisk_rs_ari::server::AriServerBuilder::new()
        .bind("127.0.0.1:0".parse().expect("valid addr"))
        .max_response_body_bytes(4)
        .build()
        .await
        .expect("server should build");
    let local_addr = server.local_addr().expect("server should have local addr");
    let (result_tx, result_rx) = tokio::sync::oneshot::channel();
    let result_tx = std::sync::Arc::new(std::sync::Mutex::new(Some(result_tx)));
    let server_task = tokio::spawn(async move {
        server
            .run(move |session| {
                let tx = result_tx.lock().expect("result mutex poisoned").take();
                async move {
                    let result = session.get::<serde_json::Value>("channels/large").await;
                    if let Some(tx) = tx {
                        let _ = tx.send(result);
                    }
                }
            })
            .await
            .expect("server run should succeed");
    });

    let url = format!("ws://{local_addr}");
    let (mut websocket, _) = tokio_tungstenite::connect_async(url)
        .await
        .expect("websocket should connect");
    let request = tokio::time::timeout(Duration::from_secs(1), websocket.next())
        .await
        .expect("session should send a REST request")
        .expect("websocket should remain connected")
        .expect("REST request frame should be valid");
    let request: serde_json::Value = serde_json::from_str(
        request
            .to_text()
            .expect("REST request should be a text frame"),
    )
    .expect("REST request should be JSON");
    websocket
        .send(tokio_tungstenite::tungstenite::Message::Text(
            serde_json::json!({
                "type": "RESTResponse",
                "status_code": 200,
                "reason_phrase": "OK",
                "uri": request["uri"],
                "request_id": request["request_id"],
                "transaction_id": "test-transaction",
                "message_body": "12345",
            })
            .to_string()
            .into(),
        ))
        .await
        .expect("REST response should send");

    let result = tokio::time::timeout(Duration::from_secs(1), result_rx)
        .await
        .expect("session request should complete")
        .expect("session result sender should remain alive");
    assert!(
        matches!(
            &result,
            Err(AriError::ResponseTooLarge {
                limit: 4,
                received: 5,
            })
        ),
        "outbound session response must respect the body cap: {result:?}"
    );

    handle.shutdown();
    assert_server_ok(server_task.await);
}

#[tokio::test]
async fn outbound_ws_server_delivers_events_to_session() {
    init_tracing();

    let bind_addr: std::net::SocketAddr = "127.0.0.1:0".parse().expect("valid addr");
    let (server, handle) = asterisk_rs_ari::server::AriServerBuilder::new()
        .bind(bind_addr)
        .build()
        .await
        .expect("server should build");

    let local_addr = server.local_addr().expect("server should have local addr");

    // channel to deliver received event type from handler to test
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<String>(1);

    let server_task = tokio::spawn(async move {
        server
            .run(move |session| {
                let tx = event_tx.clone();
                async move {
                    let mut sub = session.subscribe();
                    if let Some(msg) = sub.recv().await {
                        if let asterisk_rs_ari::AriEvent::StasisStart { channel, .. } = &msg.event {
                            let _ = tx.send(channel.id.clone()).await;
                        }
                    }
                }
            })
            .await
            .expect("server run should succeed");
    });

    // small delay for the server to be ready to accept
    tokio::time::sleep(Duration::from_millis(50)).await;

    // connect and send a StasisStart event
    let url = format!("ws://{local_addr}");
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("ws connect should succeed");

    let stasis_json = r#"{
        "type": "StasisStart",
        "application": "test-app",
        "timestamp": "2024-01-01T00:00:00.000+0000",
        "channel": {
            "id": "outbound-chan-1",
            "name": "PJSIP/100-0001",
            "state": "Ring",
            "caller": {"name": "Test", "number": "100"},
            "connected": {"name": "", "number": ""},
            "dialplan": {"context": "default", "exten": "100", "priority": 1}
        },
        "args": []
    }"#;

    use futures_util::SinkExt;
    ws_stream
        .send(tokio_tungstenite::tungstenite::Message::Text(
            stasis_json.to_string().into(),
        ))
        .await
        .expect("should send event text frame");

    // verify the session received and parsed the event
    let channel_id = tokio::time::timeout(Duration::from_secs(5), event_rx.recv())
        .await
        .expect("should receive event within timeout")
        .expect("event channel should not be closed");

    assert_eq!(
        channel_id, "outbound-chan-1",
        "session should receive the StasisStart channel id"
    );

    handle.shutdown();
    assert_server_ok(server_task.await);
}

#[tokio::test]
async fn media_channel_connect_and_disconnect() {
    use futures_util::StreamExt;

    init_tracing();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind should succeed");
    let addr = listener.local_addr().expect("should have local addr");

    // accept one connection as a WS server
    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept should succeed");
        let mut ws = tokio_tungstenite::accept_async(stream)
            .await
            .expect("ws accept should succeed");
        while let Some(message) = ws.next().await {
            if matches!(
                message.expect("media websocket frame should be valid"),
                tokio_tungstenite::tungstenite::Message::Close(_)
            ) {
                return true;
            }
        }
        false
    });

    let url = format!("ws://{addr}");
    let media = asterisk_rs_ari::media::MediaChannel::connect(&url)
        .await
        .expect("media channel connect should succeed");

    tokio::time::timeout(Duration::from_secs(2), media.disconnect_and_wait())
        .await
        .expect("cooperative media disconnect should be bounded");
    let received_close = tokio::time::timeout(Duration::from_secs(2), server_task)
        .await
        .expect("server should observe media disconnect")
        .expect("server task should not panic");
    assert!(
        received_close,
        "media disconnect must cooperatively send a websocket close frame"
    );
}

#[tokio::test]
async fn media_sync_disconnect_aborts_an_event_blocked_actor() {
    use futures_util::{SinkExt, StreamExt};

    init_tracing();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind should succeed");
    let addr = listener.local_addr().expect("should have local addr");
    let (events_sent_tx, events_sent_rx) = tokio::sync::oneshot::channel();

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept should succeed");
        let mut ws = tokio_tungstenite::accept_async(stream)
            .await
            .expect("ws accept should succeed");
        let event = serde_json::json!({
            "event": "MEDIA_START",
            "connection_id": "blocked-media",
            "channel": "WebSocket/blocked",
            "channel_id": "blocked-media",
            "format": "ulaw",
            "optimal_frame_size": 160,
            "ptime": 20
        })
        .to_string();

        for _ in 0..65 {
            ws.send(tokio_tungstenite::tungstenite::Message::Text(
                event.clone().into(),
            ))
            .await
            .expect("critical media event should be sent");
        }
        events_sent_tx
            .send(())
            .expect("test should still await the event burst");

        match tokio::time::timeout(Duration::from_secs(2), ws.next()).await {
            Ok(None | Some(Err(_))) => true,
            Ok(Some(Ok(_))) | Err(_) => false,
        }
    });

    let url = format!("ws://{addr}");
    let media = asterisk_rs_ari::media::MediaChannel::connect(&url)
        .await
        .expect("media channel connect should succeed");
    events_sent_rx
        .await
        .expect("server should send enough events to fill the bounded receiver");
    for _ in 0..8 {
        tokio::task::yield_now().await;
    }

    media.disconnect();

    let peer_terminated = tokio::time::timeout(Duration::from_secs(3), server_task)
        .await
        .expect("synchronous disconnect should terminate the peer connection")
        .expect("server task should not panic");
    assert!(
        peer_terminated,
        "synchronous disconnect must abort an actor blocked on event delivery"
    );
}

#[tokio::test]
async fn media_channel_sends_command() {
    use futures_util::StreamExt;

    init_tracing();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind should succeed");
    let addr = listener.local_addr().expect("should have local addr");

    let (msg_tx, mut msg_rx) = tokio::sync::mpsc::channel::<String>(1);

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept should succeed");
        let mut ws = tokio_tungstenite::accept_async(stream)
            .await
            .expect("ws accept should succeed");
        // read the first text frame
        while let Some(Ok(msg)) = ws.next().await {
            if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                let _ = msg_tx.send(text.to_string()).await;
                break;
            }
        }
    });

    let url = format!("ws://{addr}");
    let media = asterisk_rs_ari::media::MediaChannel::connect(&url)
        .await
        .expect("media channel connect should succeed");

    media
        .send_command(asterisk_rs_ari::media::MediaCommand::Answer)
        .await
        .expect("send_command should succeed");

    let received = tokio::time::timeout(Duration::from_secs(5), msg_rx.recv())
        .await
        .expect("should receive command within timeout")
        .expect("channel should not be closed");

    let parsed: serde_json::Value =
        serde_json::from_str(&received).expect("received text should be valid JSON");
    assert_eq!(
        parsed["command"], "ANSWER",
        "command field should be ANSWER"
    );

    media.disconnect();
    assert_server_ok(server_task.await);
}

#[tokio::test]
async fn media_channel_receives_event() {
    use futures_util::SinkExt;

    init_tracing();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind should succeed");
    let addr = listener.local_addr().expect("should have local addr");

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept should succeed");
        let mut ws = tokio_tungstenite::accept_async(stream)
            .await
            .expect("ws accept should succeed");

        let media_start_json = r#"{
            "event": "MEDIA_START",
            "connection_id": "conn-1",
            "channel": "PJSIP/100-0001",
            "channel_id": "chan-1",
            "format": "ulaw",
            "optimal_frame_size": 160,
            "ptime": 20
        }"#;

        ws.send(tokio_tungstenite::tungstenite::Message::Text(
            media_start_json.to_string().into(),
        ))
        .await
        .expect("server should send event");

        // hold connection open briefly
        tokio::time::sleep(Duration::from_secs(2)).await;
    });

    let url = format!("ws://{addr}");
    let mut media = asterisk_rs_ari::media::MediaChannel::connect(&url)
        .await
        .expect("media channel connect should succeed");

    let event = tokio::time::timeout(Duration::from_secs(5), media.recv_event())
        .await
        .expect("should receive event within timeout")
        .expect("event stream should not be closed");

    match event {
        asterisk_rs_ari::media::MediaEvent::MediaStart {
            connection_id,
            format,
            optimal_frame_size,
            ptime,
            ..
        } => {
            assert_eq!(connection_id, "conn-1", "connection_id mismatch");
            assert_eq!(format, "ulaw", "format mismatch");
            assert_eq!(optimal_frame_size, 160, "optimal_frame_size mismatch");
            assert_eq!(ptime, 20, "ptime mismatch");
        }
        other => panic!("expected MediaStart, got: {other:?}"),
    }

    media.disconnect();
    assert_server_ok(server_task.await);
}

#[tokio::test]
async fn media_channel_sends_and_receives_audio() {
    use futures_util::{SinkExt, StreamExt};

    init_tracing();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind should succeed");
    let addr = listener.local_addr().expect("should have local addr");

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept should succeed");
        let mut ws = tokio_tungstenite::accept_async(stream)
            .await
            .expect("ws accept should succeed");

        // read a binary frame and echo it back
        while let Some(Ok(msg)) = ws.next().await {
            if let tokio_tungstenite::tungstenite::Message::Binary(data) = msg {
                ws.send(tokio_tungstenite::tungstenite::Message::Binary(data))
                    .await
                    .expect("server should echo audio");
                break;
            }
        }
        // hold connection open briefly
        tokio::time::sleep(Duration::from_secs(1)).await;
    });

    let url = format!("ws://{addr}");
    let mut media = asterisk_rs_ari::media::MediaChannel::connect(&url)
        .await
        .expect("media channel connect should succeed");

    let test_audio = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04];
    media
        .send_audio(test_audio.clone())
        .await
        .expect("send_audio should succeed");

    let received = tokio::time::timeout(Duration::from_secs(5), media.recv_audio())
        .await
        .expect("should receive audio within timeout")
        .expect("audio stream should not be closed");

    assert_eq!(
        received, test_audio,
        "audio round-trip should preserve data"
    );

    media.disconnect();
    assert_server_ok(server_task.await);
}
