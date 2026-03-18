use std::time::Duration;

use asterisk_rs_ami::client::AmiClient;
use asterisk_rs_core::config::ReconnectPolicy;
use asterisk_rs_tests::helpers::init_tracing;
use asterisk_rs_tests::mock::ami_server::{get_header, handle_login, MockAmiServer};

#[tokio::test]
async fn connect_and_login() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;
        // keep connection alive until client disconnects
        loop {
            if conn.read_message().await.is_none() {
                break;
            }
        }
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect and login");

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");
    let _ = handle.await;
}

#[tokio::test]
async fn login_rejected() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        // reject challenge
        let msg = conn.read_message().await.expect("should receive challenge");
        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();
        conn.send_message(&[
            ("Response", "Error"),
            ("ActionID", &aid),
            ("Message", "Challenge not supported"),
        ])
        .await;

        // reject plaintext login too
        let msg = conn.read_message().await.expect("should receive login");
        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();
        conn.send_message(&[
            ("Response", "Error"),
            ("ActionID", &aid),
            ("Message", "Authentication failed"),
        ])
        .await;
    });

    let result = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "wrong")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await;

    assert!(result.is_err(), "login should fail when rejected");
    let _ = handle.await;
}

#[tokio::test]
async fn send_ping() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        // handle ping
        let msg = conn.read_message().await.expect("should receive ping");
        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();
        let action = get_header(&msg, "Action").expect("should have Action");
        assert_eq!(action, "Ping");

        conn.send_message(&[
            ("Response", "Success"),
            ("ActionID", &aid),
            ("Ping", "Pong"),
            ("Timestamp", "1234567890.000000"),
        ])
        .await;

        // read logoff on disconnect
        let _ = conn.read_message().await;
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    let response = client.ping().await.expect("ping should succeed");
    assert!(response.success, "ping response should be success");
    assert_eq!(response.get("Ping"), Some("Pong"));

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");
    let _ = handle.await;
}

#[tokio::test]
async fn send_action_timeout() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        // read the ping but never respond — triggers client timeout
        let _msg = conn.read_message().await;

        // hold connection open until client disconnects
        loop {
            if conn.read_message().await.is_none() {
                break;
            }
        }
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_millis(500))
        .build()
        .await
        .expect("client should connect");

    let result = client.ping().await;
    assert!(result.is_err(), "ping should timeout");
    let err = result.expect_err("already asserted err");
    let msg = format!("{err}");
    assert!(
        msg.contains("timeout"),
        "error should mention timeout: {msg}"
    );

    // drop client to close connection
    drop(client);
    let _ = handle.await;
}

#[tokio::test]
async fn receive_events() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        // send an unsolicited event
        conn.send_message(&[
            ("Event", "FullyBooted"),
            ("Privilege", "system,all"),
            ("Status", "Fully Booted"),
        ])
        .await;

        // hold connection until client drops
        loop {
            if conn.read_message().await.is_none() {
                break;
            }
        }
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    let mut sub = client.subscribe();

    // wait for the event with a timeout
    let event = tokio::time::timeout(Duration::from_secs(3), sub.recv())
        .await
        .expect("should receive event within timeout")
        .expect("subscription should not be closed");

    assert_eq!(event.event_name(), "FullyBooted");

    drop(client);
    let _ = handle.await;
}

#[tokio::test]
async fn graceful_disconnect() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        // expect Logoff action from disconnect()
        let msg = conn.read_message().await.expect("should receive logoff");
        let action = get_header(&msg, "Action").expect("should have Action");
        assert_eq!(action, "Logoff");

        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();
        conn.send_message(&[
            ("Response", "Goodbye"),
            ("ActionID", &aid),
            ("Message", "Thanks for all the fish."),
        ])
        .await;
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");

    // give the background task a moment to process shutdown
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert_eq!(
        client.connection_state(),
        asterisk_rs_core::config::ConnectionState::Disconnected,
        "should be disconnected after disconnect()"
    );

    let _ = handle.await;
}

#[tokio::test]
async fn concurrent_actions() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        // read two pings and respond to both
        let mut pings = Vec::new();
        for _ in 0..2 {
            let msg = conn.read_message().await.expect("should receive action");
            let aid = get_header(&msg, "ActionID")
                .expect("should have ActionID")
                .to_string();
            pings.push(aid);
        }

        for aid in &pings {
            conn.send_message(&[("Response", "Success"), ("ActionID", aid), ("Ping", "Pong")])
                .await;
        }

        // drain remaining (logoff)
        loop {
            if conn.read_message().await.is_none() {
                break;
            }
        }
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    // fire two pings concurrently
    let c1 = client.clone();
    let c2 = client.clone();
    let (r1, r2) = tokio::join!(
        tokio::spawn(async move { c1.ping().await }),
        tokio::spawn(async move { c2.ping().await }),
    );

    let resp1 = r1
        .expect("task should not panic")
        .expect("ping 1 should succeed");
    let resp2 = r2
        .expect("task should not panic")
        .expect("ping 2 should succeed");
    assert!(resp1.success);
    assert!(resp2.success);
    // action IDs should differ
    assert_ne!(resp1.action_id, resp2.action_id);

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");
    let _ = handle.await;
}

#[tokio::test]
async fn md5_challenge_auth() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        // read Challenge
        let msg = conn.read_message().await.expect("should receive challenge");
        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();
        conn.send_message(&[
            ("Response", "Success"),
            ("ActionID", &aid),
            ("Challenge", "abcdef1234"),
        ])
        .await;

        // read Login — verify it uses Key (md5), not Secret (plaintext)
        let msg = conn.read_message().await.expect("should receive login");
        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();
        assert!(
            get_header(&msg, "Key").is_some(),
            "login should contain Key header for md5 auth"
        );
        assert!(
            get_header(&msg, "Secret").is_none(),
            "login should not contain Secret when using md5"
        );
        assert_eq!(
            get_header(&msg, "AuthType"),
            Some("md5"),
            "login should specify AuthType md5"
        );

        conn.send_message(&[
            ("Response", "Success"),
            ("ActionID", &aid),
            ("Message", "Authentication accepted"),
        ])
        .await;
        // keep connection alive until client disconnects
        loop {
            if conn.read_message().await.is_none() {
                break;
            }
        }
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("md5 auth should succeed");

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");
    let _ = handle.await;
}

#[tokio::test]
async fn plaintext_fallback() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        // reject challenge
        let msg = conn.read_message().await.expect("should receive challenge");
        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();
        conn.send_message(&[
            ("Response", "Error"),
            ("ActionID", &aid),
            ("Message", "Challenge not supported"),
        ])
        .await;

        // accept plaintext login — verify it uses Secret, not Key
        let msg = conn.read_message().await.expect("should receive login");
        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();
        assert!(
            get_header(&msg, "Secret").is_some(),
            "plaintext login should contain Secret"
        );
        assert!(
            get_header(&msg, "Key").is_none(),
            "plaintext login should not contain Key"
        );

        conn.send_message(&[
            ("Response", "Success"),
            ("ActionID", &aid),
            ("Message", "Authentication accepted"),
        ])
        .await;
        // keep connection alive until client disconnects
        loop {
            if conn.read_message().await.is_none() {
                break;
            }
        }
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "mysecret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("plaintext fallback auth should succeed");

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");
    let _ = handle.await;
}

#[tokio::test]
async fn command_response() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        // read Command action
        let msg = conn.read_message().await.expect("should receive command");
        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();
        let action = get_header(&msg, "Action").expect("should have Action");
        assert_eq!(action, "Command");

        // respond with Response: Follows format —
        // the codec expects headers + output lines + --END COMMAND-- all within one
        // \r\n\r\n delimited block
        conn.send_raw(
            format!(
                "Response: Follows\r\n\
                 ActionID: {aid}\r\n\
                 Asterisk 22.0.0 on x86_64 running Linux\r\n\
                 Built on 2024-01-01\r\n\
                 --END COMMAND--\r\n\
                 \r\n"
            )
            .as_bytes(),
        )
        .await;

        // drain remaining
        loop {
            if conn.read_message().await.is_none() {
                break;
            }
        }
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    let response = client
        .command("core show version")
        .await
        .expect("command should succeed");
    assert!(response.success, "command response should be success");
    assert_eq!(response.response_type, "Follows");
    assert!(
        !response.output.is_empty(),
        "command output should not be empty"
    );

    // verify the output contains version info
    let has_version = response.output.iter().any(|line| line.contains("Asterisk"));
    assert!(
        has_version,
        "output should contain Asterisk version: {:?}",
        response.output
    );

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");
    let _ = handle.await;
}

#[tokio::test]
async fn reconnect_disabled() {
    init_tracing();

    // no server running — connection should fail immediately with ReconnectPolicy::none()
    let result = AmiClient::builder()
        .host("127.0.0.1")
        .port(19999) // nothing listening
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(2))
        .build()
        .await;

    assert!(
        result.is_err(),
        "should fail when no server is running with no reconnect"
    );
}

#[tokio::test]
async fn filtered_subscription() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        // send a mix of events — only Hangup should pass the filter
        conn.send_message(&[("Event", "FullyBooted"), ("Status", "Fully Booted")])
            .await;
        conn.send_message(&[
            ("Event", "Hangup"),
            ("Channel", "SIP/100-00000001"),
            ("Uniqueid", "999.1"),
            ("Cause", "16"),
            ("Cause-txt", "Normal Clearing"),
        ])
        .await;
        conn.send_message(&[("Event", "FullyBooted"), ("Status", "Fully Booted")])
            .await;

        loop {
            if conn.read_message().await.is_none() {
                break;
            }
        }
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    let mut filtered = client.subscribe_filtered(|e| e.event_name() == "Hangup");

    let event = tokio::time::timeout(Duration::from_secs(3), filtered.recv())
        .await
        .expect("should receive filtered event within timeout")
        .expect("subscription should not be closed");

    assert_eq!(event.event_name(), "Hangup");
    if let asterisk_rs_ami::AmiEvent::Hangup {
        cause, cause_txt, ..
    } = &event
    {
        assert_eq!(*cause, 16);
        assert_eq!(cause_txt, "Normal Clearing");
    } else {
        panic!("expected Hangup variant, got: {event:?}");
    }

    drop(client);
    let _ = handle.await;
}

#[tokio::test]
async fn send_collecting_event_list() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        // read the Status action
        let msg = conn
            .read_message()
            .await
            .expect("should receive Status action");
        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();
        let action = get_header(&msg, "Action").expect("should have Action");
        assert_eq!(action, "Status");

        // send initial response
        conn.send_message(&[
            ("Response", "Success"),
            ("ActionID", &aid),
            ("Message", "Channel status will follow"),
        ])
        .await;

        // send a Status event (one channel)
        conn.send_message(&[
            ("Event", "Status"),
            ("ActionID", &aid),
            ("Channel", "SIP/100-00000001"),
            ("ChannelState", "6"),
            ("ChannelStateDesc", "Up"),
            ("Uniqueid", "111.1"),
        ])
        .await;

        // send a second Status event
        conn.send_message(&[
            ("Event", "Status"),
            ("ActionID", &aid),
            ("Channel", "SIP/200-00000002"),
            ("ChannelState", "4"),
            ("ChannelStateDesc", "Ring"),
            ("Uniqueid", "111.2"),
        ])
        .await;

        // send the completion event
        conn.send_message(&[
            ("Event", "StatusComplete"),
            ("ActionID", &aid),
            ("Items", "2"),
        ])
        .await;

        // drain remaining
        loop {
            if conn.read_message().await.is_none() {
                break;
            }
        }
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    let result = client
        .send_collecting(&asterisk_rs_ami::action::StatusAction { channel: None })
        .await
        .expect("send_collecting should succeed");

    assert!(
        result.response.success,
        "initial response should be success"
    );
    // completion event is included in the list
    assert_eq!(
        result.events.len(),
        3,
        "should have 2 Status + 1 StatusComplete events"
    );

    // first two are Status events
    assert_eq!(result.events[0].event_name(), "Status");
    assert_eq!(result.events[1].event_name(), "Status");
    assert_eq!(result.events[2].event_name(), "StatusComplete");

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");
    let _ = handle.await;
}

#[tokio::test]
async fn channel_variables_in_response() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        // read any action
        let msg = conn.read_message().await.expect("should receive action");
        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();

        // respond with ChanVariable headers
        conn.send_raw(
            format!(
                "Response: Success\r\n\
                 ActionID: {aid}\r\n\
                 ChanVariable(DIALSTATUS): ANSWER\r\n\
                 ChanVariable(ANSWEREDTIME): 42\r\n\
                 \r\n"
            )
            .as_bytes(),
        )
        .await;

        loop {
            if conn.read_message().await.is_none() {
                break;
            }
        }
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    let response = client.ping().await.expect("action should succeed");
    assert!(response.success);
    assert_eq!(response.get_variable("DIALSTATUS"), Some("ANSWER"));
    assert_eq!(response.get_variable("ANSWEREDTIME"), Some("42"));

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");
    let _ = handle.await;
}

#[tokio::test]
async fn reconnect_on_disconnect() {
    init_tracing();

    // single listener that will accept two connections:
    // first: login then crash. second: login then handle ping.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let port = listener.local_addr().expect("addr").port();

    // spawn the mock server side
    let mock_handle = tokio::spawn(async move {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        // helper to read one AMI message from client
        async fn read_msg(
            reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
        ) -> Vec<(String, String)> {
            let mut headers = Vec::new();
            loop {
                let mut line = String::new();
                let n = reader.read_line(&mut line).await.expect("read");
                if n == 0 {
                    return headers;
                }
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    if !headers.is_empty() {
                        return headers;
                    }
                    continue;
                }
                if let Some((k, v)) = trimmed.split_once(':') {
                    headers.push((k.trim().to_string(), v.trim().to_string()));
                }
            }
        }

        // helper to do challenge-response login
        async fn do_login(
            reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
            writer: &mut tokio::net::tcp::OwnedWriteHalf,
        ) {
            let msg = read_msg(reader).await;
            let aid = msg
                .iter()
                .find(|(k, _)| k == "ActionID")
                .map(|(_, v)| v.as_str())
                .expect("ActionID");
            let resp = format!("Response: Success\r\nActionID: {aid}\r\nChallenge: abc\r\n\r\n");
            writer.write_all(resp.as_bytes()).await.expect("write");

            let msg = read_msg(reader).await;
            let aid = msg
                .iter()
                .find(|(k, _)| k == "ActionID")
                .map(|(_, v)| v.as_str())
                .expect("ActionID");
            let resp = format!("Response: Success\r\nActionID: {aid}\r\nMessage: OK\r\n\r\n");
            writer.write_all(resp.as_bytes()).await.expect("write");
        }

        // --- connection 1: login then crash ---
        let (stream, _) = listener.accept().await.expect("accept 1");
        let (read, mut write) = stream.into_split();
        write
            .write_all(b"Asterisk Call Manager/6.0.0\r\n")
            .await
            .expect("banner 1");
        let mut reader = BufReader::new(read);
        do_login(&mut reader, &mut write).await;
        // drop connection to simulate crash
        drop(reader);
        drop(write);

        // --- connection 2: login + serve ping ---
        let (stream, _) = listener.accept().await.expect("accept 2 (reconnect)");
        let (read, mut write) = stream.into_split();
        write
            .write_all(b"Asterisk Call Manager/6.0.0\r\n")
            .await
            .expect("banner 2");
        let mut reader = BufReader::new(read);
        do_login(&mut reader, &mut write).await;

        // handle ping
        let msg = read_msg(&mut reader).await;
        let aid = msg
            .iter()
            .find(|(k, _)| k == "ActionID")
            .map(|(_, v)| v.as_str())
            .expect("ActionID");
        let resp = format!("Response: Success\r\nActionID: {aid}\r\nPing: Pong\r\n\r\n");
        write.write_all(resp.as_bytes()).await.expect("pong");

        // drain until client disconnects
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) | Err(_) => break,
                _ => {}
            }
        }
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::fixed(Duration::from_millis(100)))
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("initial connect should succeed");

    // wait for connection to drop and reconnect to complete
    // the client auto-reconnects in the background
    tokio::time::sleep(Duration::from_secs(1)).await;

    // after reconnect, ping should work
    let response = client
        .ping()
        .await
        .expect("ping after reconnect should succeed");
    assert!(response.success, "ping response should be success");
    assert_eq!(response.get("Ping"), Some("Pong"));

    client.disconnect().await.expect("disconnect");
    let _ = mock_handle.await;
}

#[tokio::test]
async fn builder_missing_credentials() {
    init_tracing();

    let result = AmiClient::builder()
        .host("127.0.0.1")
        .port(9999)
        .build()
        .await;

    assert!(result.is_err(), "builder without credentials should fail");
}

#[tokio::test]
async fn connection_state_transitions() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;
        loop {
            if conn.read_message().await.is_none() {
                break;
            }
        }
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    assert_eq!(
        client.connection_state(),
        asterisk_rs_core::config::ConnectionState::Connected,
        "should be connected after build"
    );

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");

    tokio::time::sleep(Duration::from_millis(100)).await;

    assert_eq!(
        client.connection_state(),
        asterisk_rs_core::config::ConnectionState::Disconnected,
        "should be disconnected after disconnect"
    );

    let _ = handle.await;
}

#[tokio::test]
async fn concurrent_stress_50_actions() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        // read and respond to 50 actions
        for _ in 0..50 {
            let msg = conn.read_message().await.expect("should receive action");
            let aid = get_header(&msg, "ActionID")
                .expect("should have ActionID")
                .to_string();
            conn.send_message(&[
                ("Response", "Success"),
                ("ActionID", &aid),
                ("Ping", "Pong"),
            ])
            .await;
        }

        // drain remaining (logoff)
        loop {
            if conn.read_message().await.is_none() {
                break;
            }
        }
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .await
        .expect("client should connect");

    // spawn 50 concurrent pings
    let mut handles = Vec::new();
    for _ in 0..50 {
        let c = client.clone();
        handles.push(tokio::spawn(async move { c.ping().await }));
    }

    let mut action_ids = std::collections::HashSet::new();
    for (i, h) in handles.into_iter().enumerate() {
        let resp = h
            .await
            .expect("task should not panic")
            .unwrap_or_else(|e| panic!("ping {i} should succeed: {e}"));
        assert!(resp.success, "ping {i} response should be success");
        action_ids.insert(resp.action_id.clone());
    }

    assert_eq!(action_ids.len(), 50, "all 50 action IDs should be distinct");

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");
    let _ = handle.await;
}

#[tokio::test]
async fn connection_drop_cancels_pending_actions() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        // read the ping but drop connection immediately without responding
        let _msg = conn.read_message().await;
        // connection drops here when conn is dropped
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    let result = client.ping().await;
    assert!(
        result.is_err(),
        "ping should fail when connection is dropped"
    );

    let _ = handle.await;
}

#[tokio::test]
async fn multiple_subscribers_all_receive() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        // small delay so subscribers are registered before the event arrives
        tokio::time::sleep(Duration::from_millis(100)).await;

        conn.send_message(&[
            ("Event", "FullyBooted"),
            ("Privilege", "system,all"),
            ("Status", "Fully Booted"),
        ])
        .await;

        loop {
            if conn.read_message().await.is_none() {
                break;
            }
        }
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    let mut sub1 = client.subscribe();
    let mut sub2 = client.subscribe();
    let mut sub3 = client.subscribe();

    let timeout = Duration::from_secs(3);
    let e1 = tokio::time::timeout(timeout, sub1.recv())
        .await
        .expect("sub1 should receive event within timeout")
        .expect("sub1 should not be closed");
    let e2 = tokio::time::timeout(timeout, sub2.recv())
        .await
        .expect("sub2 should receive event within timeout")
        .expect("sub2 should not be closed");
    let e3 = tokio::time::timeout(timeout, sub3.recv())
        .await
        .expect("sub3 should receive event within timeout")
        .expect("sub3 should not be closed");

    assert_eq!(e1.event_name(), "FullyBooted");
    assert_eq!(e2.event_name(), "FullyBooted");
    assert_eq!(e3.event_name(), "FullyBooted");

    drop(client);
    let _ = handle.await;
}

#[tokio::test]
async fn ping_interval_sends_periodic_pings() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<u32>(1);

    let handle = server.accept_one(move |mut conn| async move {
        handle_login(&mut conn).await;

        let mut ping_count: u32 = 0;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(1);

        // read messages for 1 second, counting pings
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, conn.read_message()).await {
                Ok(Some(msg)) => {
                    if get_header(&msg, "Action") == Some("Ping") {
                        let aid = get_header(&msg, "ActionID")
                            .expect("ping should have ActionID")
                            .to_string();
                        conn.send_message(&[
                            ("Response", "Success"),
                            ("ActionID", &aid),
                            ("Ping", "Pong"),
                        ])
                        .await;
                        ping_count += 1;
                    }
                }
                Ok(None) => break, // eof
                Err(_) => break,   // timeout
            }
        }

        tx.send(ping_count).await.expect("should send count");

        // drain until client disconnects
        loop {
            if conn.read_message().await.is_none() {
                break;
            }
        }
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .ping_interval(Duration::from_millis(200))
        .build()
        .await
        .expect("client should connect");

    let count = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("should receive count within timeout")
        .expect("channel should not be closed");

    assert!(
        count >= 3,
        "expected at least 3 periodic pings in 1s at 200ms interval, got {count}"
    );

    drop(client);
    let _ = handle.await;
}

#[tokio::test]
async fn send_on_disconnected_client_returns_error() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;
        // read logoff then drain
        loop {
            if conn.read_message().await.is_none() {
                break;
            }
        }
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let result = client.ping().await;
    assert!(result.is_err(), "ping on disconnected client should fail");

    let _ = handle.await;
}

#[tokio::test]
async fn originate_action_success() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        let msg = conn.read_message().await.expect("should receive originate");
        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();
        let action = get_header(&msg, "Action").expect("should have Action");
        assert_eq!(action, "Originate");
        assert_eq!(get_header(&msg, "Channel"), Some("SIP/100@trunk"));
        assert_eq!(get_header(&msg, "Context"), Some("default"));
        assert_eq!(get_header(&msg, "Exten"), Some("200"));
        assert_eq!(get_header(&msg, "Priority"), Some("1"));

        conn.send_message(&[
            ("Response", "Success"),
            ("ActionID", &aid),
            ("Message", "Originate successfully queued"),
        ])
        .await;

        while conn.read_message().await.is_some() {}
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    let action = asterisk_rs_ami::action::OriginateAction::new("SIP/100@trunk")
        .context("default")
        .extension("200")
        .priority(1);
    let response = client
        .originate(action)
        .await
        .expect("originate should succeed");
    assert!(response.success, "originate response should be success");
    assert_eq!(
        response.message.as_deref(),
        Some("Originate successfully queued")
    );

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");
    let _ = handle.await;
}

#[tokio::test]
async fn originate_action_failure() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        let msg = conn.read_message().await.expect("should receive originate");
        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();

        conn.send_message(&[
            ("Response", "Error"),
            ("ActionID", &aid),
            ("Message", "Channel not found"),
        ])
        .await;

        while conn.read_message().await.is_some() {}
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    let action = asterisk_rs_ami::action::OriginateAction::new("SIP/nonexistent");
    let response = client.originate(action).await.expect("should get response");
    assert!(!response.success, "originate should fail");
    assert_eq!(response.message.as_deref(), Some("Channel not found"));

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");
    let _ = handle.await;
}

#[tokio::test]
async fn hangup_action_success() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        let msg = conn.read_message().await.expect("should receive hangup");
        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();
        let action = get_header(&msg, "Action").expect("should have Action");
        assert_eq!(action, "Hangup");
        assert_eq!(get_header(&msg, "Channel"), Some("SIP/100-00000001"));

        conn.send_message(&[
            ("Response", "Success"),
            ("ActionID", &aid),
            ("Message", "Channel Hungup"),
        ])
        .await;

        while conn.read_message().await.is_some() {}
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    let action = asterisk_rs_ami::action::HangupAction::new("SIP/100-00000001");
    let response = client.hangup(action).await.expect("hangup should succeed");
    assert!(response.success, "hangup response should be success");

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");
    let _ = handle.await;
}

#[tokio::test]
async fn hangup_action_with_cause() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        let msg = conn.read_message().await.expect("should receive hangup");
        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();
        assert_eq!(get_header(&msg, "Action"), Some("Hangup"));
        assert_eq!(get_header(&msg, "Channel"), Some("SIP/200-00000002"));
        assert_eq!(
            get_header(&msg, "Cause"),
            Some("21"),
            "cause header should be sent to server"
        );

        conn.send_message(&[("Response", "Success"), ("ActionID", &aid)])
            .await;

        while conn.read_message().await.is_some() {}
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    let action = asterisk_rs_ami::action::HangupAction::new("SIP/200-00000002").cause(21);
    let response = client.hangup(action).await.expect("hangup should succeed");
    assert!(response.success);

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");
    let _ = handle.await;
}

#[tokio::test]
async fn send_collecting_status() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        let msg = conn
            .read_message()
            .await
            .expect("should receive Status action");
        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();
        assert_eq!(get_header(&msg, "Action"), Some("Status"));

        conn.send_message(&[
            ("Response", "Success"),
            ("ActionID", &aid),
            ("Message", "Channel status will follow"),
        ])
        .await;

        // one status event
        conn.send_message(&[
            ("Event", "Status"),
            ("ActionID", &aid),
            ("Channel", "PJSIP/300-00000003"),
            ("ChannelState", "6"),
            ("ChannelStateDesc", "Up"),
        ])
        .await;

        // completion
        conn.send_message(&[
            ("Event", "StatusComplete"),
            ("ActionID", &aid),
            ("Items", "1"),
        ])
        .await;

        while conn.read_message().await.is_some() {}
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    let result = client
        .send_collecting(&asterisk_rs_ami::action::StatusAction { channel: None })
        .await
        .expect("send_collecting should succeed");

    assert!(result.response.success);
    assert_eq!(
        result.events.len(),
        2,
        "should have 1 Status + 1 StatusComplete"
    );
    assert_eq!(result.events[0].event_name(), "Status");
    assert_eq!(result.events[1].event_name(), "StatusComplete");

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");
    let _ = handle.await;
}

#[tokio::test]
async fn send_collecting_core_show_channels() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        let msg = conn.read_message().await.expect("should receive action");
        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();
        assert_eq!(get_header(&msg, "Action"), Some("CoreShowChannels"));

        conn.send_message(&[
            ("Response", "Success"),
            ("ActionID", &aid),
            ("Message", "Channels will follow"),
        ])
        .await;

        conn.send_message(&[
            ("Event", "CoreShowChannel"),
            ("ActionID", &aid),
            ("Channel", "SIP/100-00000001"),
            ("Duration", "00:01:30"),
        ])
        .await;

        conn.send_message(&[
            ("Event", "CoreShowChannel"),
            ("ActionID", &aid),
            ("Channel", "SIP/200-00000002"),
            ("Duration", "00:05:00"),
        ])
        .await;

        conn.send_message(&[
            ("Event", "CoreShowChannelsComplete"),
            ("ActionID", &aid),
            ("ListItems", "2"),
        ])
        .await;

        while conn.read_message().await.is_some() {}
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    let result = client
        .send_collecting(&asterisk_rs_ami::action::CoreShowChannelsAction)
        .await
        .expect("send_collecting should succeed");

    assert!(result.response.success);
    assert_eq!(
        result.events.len(),
        3,
        "should have 2 CoreShowChannel + 1 CoreShowChannelsComplete"
    );
    assert_eq!(result.events[0].event_name(), "CoreShowChannel");
    assert_eq!(result.events[1].event_name(), "CoreShowChannel");
    assert_eq!(result.events[2].event_name(), "CoreShowChannelsComplete");

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");
    let _ = handle.await;
}

#[tokio::test]
async fn send_collecting_queue_status() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        let msg = conn.read_message().await.expect("should receive action");
        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();
        assert_eq!(get_header(&msg, "Action"), Some("QueueStatus"));

        conn.send_message(&[
            ("Response", "Success"),
            ("ActionID", &aid),
            ("Message", "Queue status will follow"),
        ])
        .await;

        conn.send_message(&[
            ("Event", "QueueParams"),
            ("ActionID", &aid),
            ("Queue", "support"),
            ("Calls", "3"),
        ])
        .await;

        conn.send_message(&[
            ("Event", "QueueMember"),
            ("ActionID", &aid),
            ("Queue", "support"),
            ("Name", "Agent/1001"),
        ])
        .await;

        conn.send_message(&[
            ("Event", "QueueStatusComplete"),
            ("ActionID", &aid),
            ("Items", "2"),
        ])
        .await;

        while conn.read_message().await.is_some() {}
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    let result = client
        .send_collecting(&asterisk_rs_ami::action::QueueStatusAction {
            queue: None,
            member: None,
        })
        .await
        .expect("send_collecting should succeed");

    assert!(result.response.success);
    assert_eq!(
        result.events.len(),
        3,
        "should have QueueParams + QueueMember + QueueStatusComplete"
    );
    assert_eq!(result.events[0].event_name(), "QueueParams");
    assert_eq!(result.events[1].event_name(), "QueueMember");
    assert_eq!(result.events[2].event_name(), "QueueStatusComplete");

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");
    let _ = handle.await;
}

#[tokio::test]
async fn send_collecting_no_events() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        let msg = conn.read_message().await.expect("should receive action");
        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();

        conn.send_message(&[
            ("Response", "Success"),
            ("ActionID", &aid),
            ("Message", "Channel status will follow"),
        ])
        .await;

        // immediately send complete with 0 items
        conn.send_message(&[
            ("Event", "StatusComplete"),
            ("ActionID", &aid),
            ("Items", "0"),
        ])
        .await;

        while conn.read_message().await.is_some() {}
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    let result = client
        .send_collecting(&asterisk_rs_ami::action::StatusAction { channel: None })
        .await
        .expect("send_collecting should succeed");

    assert!(result.response.success);
    assert_eq!(
        result.events.len(),
        1,
        "should have only the StatusComplete event"
    );
    assert_eq!(result.events[0].event_name(), "StatusComplete");

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");
    let _ = handle.await;
}

#[tokio::test]
async fn action_failed_response_with_message() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        let msg = conn.read_message().await.expect("should receive action");
        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();

        conn.send_message(&[
            ("Response", "Error"),
            ("ActionID", &aid),
            ("Message", "Permission denied"),
        ])
        .await;

        while conn.read_message().await.is_some() {}
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    let response = client.ping().await.expect("should get response");
    assert!(!response.success, "response should indicate failure");
    assert_eq!(response.response_type, "Error");
    assert_eq!(response.message.as_deref(), Some("Permission denied"));

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");
    let _ = handle.await;
}

#[tokio::test]
async fn action_timeout_on_no_response() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        // read the action but never respond
        let _msg = conn.read_message().await;

        // hold connection open
        loop {
            if conn.read_message().await.is_none() {
                break;
            }
        }
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_millis(100))
        .build()
        .await
        .expect("client should connect");

    let result = client.ping().await;
    assert!(result.is_err(), "action should timeout");
    let err = format!("{}", result.expect_err("already asserted err"));
    assert!(
        err.contains("timeout"),
        "error should mention timeout: {err}"
    );

    drop(client);
    let _ = handle.await;
}

#[tokio::test]
async fn concurrent_actions_interleaved_responses() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        // read 3 actions, respond in reverse order
        let mut actions = Vec::new();
        for _ in 0..3 {
            let msg = conn.read_message().await.expect("should receive action");
            let aid = get_header(&msg, "ActionID")
                .expect("should have ActionID")
                .to_string();
            actions.push(aid);
        }

        // respond in reverse order
        for (i, aid) in actions.iter().rev().enumerate() {
            conn.send_message(&[
                ("Response", "Success"),
                ("ActionID", aid),
                ("Ping", "Pong"),
                ("Order", &(i + 1).to_string()),
            ])
            .await;
        }

        while conn.read_message().await.is_some() {}
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    let c1 = client.clone();
    let c2 = client.clone();
    let c3 = client.clone();

    let (r1, r2, r3) = tokio::join!(
        tokio::spawn(async move { c1.ping().await }),
        tokio::spawn(async move { c2.ping().await }),
        tokio::spawn(async move { c3.ping().await }),
    );

    let resp1 = r1
        .expect("task 1 should not panic")
        .expect("ping 1 should succeed");
    let resp2 = r2
        .expect("task 2 should not panic")
        .expect("ping 2 should succeed");
    let resp3 = r3
        .expect("task 3 should not panic")
        .expect("ping 3 should succeed");

    assert!(resp1.success);
    assert!(resp2.success);
    assert!(resp3.success);

    // all three should have distinct action IDs
    let mut ids = vec![&resp1.action_id, &resp2.action_id, &resp3.action_id];
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 3, "all 3 action IDs should be distinct");

    client
        .disconnect()
        .await
        .expect("disconnect should succeed");
    let _ = handle.await;
}

#[tokio::test]
async fn event_bus_continues_during_action() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        let msg = conn.read_message().await.expect("should receive action");
        let aid = get_header(&msg, "ActionID")
            .expect("should have ActionID")
            .to_string();

        // send an unsolicited event BEFORE responding to the action
        conn.send_message(&[
            ("Event", "PeerStatus"),
            ("Peer", "SIP/100"),
            ("PeerStatus", "Registered"),
        ])
        .await;

        // small delay to ensure event is dispatched before response
        tokio::time::sleep(Duration::from_millis(50)).await;

        conn.send_message(&[
            ("Response", "Success"),
            ("ActionID", &aid),
            ("Ping", "Pong"),
        ])
        .await;

        while conn.read_message().await.is_some() {}
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    let mut sub = client.subscribe();

    // send ping — the event should arrive while we wait for the response
    let response = client.ping().await.expect("ping should succeed");
    assert!(response.success);

    // the event should have been delivered to the subscriber
    let event = tokio::time::timeout(Duration::from_secs(3), sub.recv())
        .await
        .expect("should receive event within timeout")
        .expect("subscription should not be closed");
    assert_eq!(event.event_name(), "PeerStatus");

    drop(client);
    let _ = handle.await;
}

#[tokio::test]
async fn connection_refused() {
    init_tracing();

    // use a port with nothing listening
    let result = AmiClient::builder()
        .host("127.0.0.1")
        .port(19998)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(2))
        .build()
        .await;

    assert!(
        result.is_err(),
        "connecting to a port with no listener should fail"
    );
}

#[tokio::test]
async fn server_closes_during_action() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        // read the action then close the connection without responding
        let _msg = conn.read_message().await;
        // conn drops here — TCP close
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await
        .expect("client should connect");

    let result = client.ping().await;
    assert!(
        result.is_err(),
        "action should fail when server closes connection"
    );

    let _ = handle.await;
}

#[tokio::test]
async fn rapid_disconnect_reconnect_disabled() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;
        // drop connection immediately after login
    });

    let result = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .await;

    match result {
        Ok(client) => {
            // build succeeded before disconnect propagated — wait for it
            tokio::time::sleep(Duration::from_millis(300)).await;
            assert_eq!(
                client.connection_state(),
                asterisk_rs_core::config::ConnectionState::Disconnected,
                "should be disconnected after server drops with no reconnect"
            );
        }
        Err(_) => {
            // build failed because disconnect arrived first — also valid
        }
    }

    let _ = handle.await;
}

#[tokio::test]
async fn builder_custom_event_capacity() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        // send several events to test that the small capacity works
        for i in 0..5 {
            conn.send_message(&[("Event", "FullyBooted"), ("Status", &format!("Boot {i}"))])
                .await;
        }

        loop {
            if conn.read_message().await.is_none() {
                break;
            }
        }
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(5))
        .event_capacity(8)
        .build()
        .await
        .expect("client should connect with custom event capacity");

    let mut sub = client.subscribe();

    // receive at least one event to verify the bus works at custom capacity
    let event = tokio::time::timeout(Duration::from_secs(3), sub.recv())
        .await
        .expect("should receive event within timeout")
        .expect("subscription should not be closed");
    assert_eq!(event.event_name(), "FullyBooted");

    drop(client);
    let _ = handle.await;
}

#[tokio::test]
async fn builder_custom_timeout() {
    init_tracing();

    let server = MockAmiServer::start().await;
    let port = server.port();

    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;

        // read the action then delay longer than the client timeout
        let _msg = conn.read_message().await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        // hold open
        loop {
            if conn.read_message().await.is_none() {
                break;
            }
        }
    });

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_millis(50))
        .build()
        .await
        .expect("client should connect");

    let result = client.ping().await;
    assert!(result.is_err(), "action should timeout with 50ms timeout");
    let err = format!("{}", result.expect_err("already asserted err"));
    assert!(
        err.contains("timeout"),
        "error should mention timeout: {err}"
    );

    drop(client);
    let _ = handle.await;
}
