mod common;
mod mock;

use std::time::Duration;

use asterisk_rs_ami::client::AmiClient;
use asterisk_rs_core::config::ReconnectPolicy;
use mock::ami_server::{get_header, handle_login, MockAmiServer};

#[tokio::test]
async fn connect_and_login() {
    common::init_tracing();

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
    common::init_tracing();

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
    common::init_tracing();

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
    common::init_tracing();

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
    common::init_tracing();

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
    common::init_tracing();

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
    common::init_tracing();

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
    common::init_tracing();

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
    common::init_tracing();

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
    common::init_tracing();

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
    common::init_tracing();

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
    common::init_tracing();

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
    common::init_tracing();

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
    common::init_tracing();

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
    common::init_tracing();

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
