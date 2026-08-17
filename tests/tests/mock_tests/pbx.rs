use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use asterisk_rs::pbx::{DialOptions, Pbx, PbxError};
use asterisk_rs_ami::AmiClient;
use asterisk_rs_core::config::ReconnectPolicy;
use asterisk_rs_tests::helpers::assert_server_ok;
use asterisk_rs_tests::mock::ami_server::{
    MockAmiConnection, MockAmiServer, get_header, handle_login,
};
use tokio::sync::Notify;

async fn connect_client(port: u16, event_capacity: usize) -> AmiClient {
    AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(2))
        .event_capacity(event_capacity)
        .build()
        .await
        .expect("AMI client")
}

async fn accept_originate(conn: &mut MockAmiConnection) -> (Vec<(String, String)>, String) {
    let message = conn.read_message().await.expect("originate action");
    let action_id = get_header(&message, "ActionID")
        .expect("ActionID")
        .to_owned();
    conn.send_message(&[
        ("Response", "Success"),
        ("ActionID", &action_id),
        ("Message", "Originate successfully queued"),
    ])
    .await;
    (message, action_id)
}

async fn send_originate_event(
    conn: &mut MockAmiConnection,
    action_id: &str,
    response: &str,
    channel: &str,
    unique_id: &str,
) {
    conn.send_message(&[
        ("Event", "OriginateResponse"),
        ("ActionID", action_id),
        ("Response", response),
        ("Channel", channel),
        ("Uniqueid", unique_id),
        ("Reason", "4"),
    ])
    .await;
}

async fn disconnect(client: &AmiClient, handle: tokio::task::JoinHandle<()>) {
    client.disconnect().await.expect("disconnect");
    assert_server_ok(handle.await);
}

#[tokio::test]
async fn dial_sends_exact_originate_correlates_and_buffers_answer_then_hangs_up() {
    let server = MockAmiServer::start().await;
    let port = server.port();
    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;
        let (message, action_id) = accept_originate(&mut conn).await;

        assert_eq!(
            message.len(),
            11,
            "unexpected Originate headers: {message:?}"
        );
        assert_eq!(message[0], ("Action".to_owned(), "Originate".to_owned()));
        assert_eq!(message[1].0, "ActionID");
        assert!(!message[1].1.is_empty());
        assert_eq!(get_header(&message, "Channel"), Some("PJSIP/100"));
        assert_eq!(get_header(&message, "Context"), Some("default"));
        assert_eq!(get_header(&message, "Exten"), Some("200"));
        assert_eq!(get_header(&message, "Priority"), Some("1"));
        assert_eq!(get_header(&message, "Timeout"), Some("1200"));
        assert_eq!(get_header(&message, "CallerID"), Some("Test <1234>"));
        assert_eq!(get_header(&message, "Async"), Some("true"));
        let mut variables: Vec<_> = message
            .iter()
            .filter(|(name, _)| name == "Variable")
            .map(|(_, value)| value.as_str())
            .collect();
        variables.sort_unstable();
        assert_eq!(variables, ["ACCOUNT=alpha", "TRACE_ID=trace-7"]);

        send_originate_event(
            &mut conn,
            "unrelated-action",
            "Success",
            "PJSIP/wrong-00000000",
            "wrong-uid",
        )
        .await;
        conn.send_message(&[
            ("Event", "Newstate"),
            ("Channel", "PJSIP/100-00000001"),
            ("ChannelState", "6"),
            ("ChannelStateDesc", "Up"),
            ("Uniqueid", "call-uid"),
        ])
        .await;
        send_originate_event(
            &mut conn,
            &action_id,
            "Success",
            "PJSIP/100-00000001",
            "call-uid",
        )
        .await;

        let hangup = conn.read_message().await.expect("hangup action");
        assert_eq!(hangup.len(), 3, "unexpected Hangup headers: {hangup:?}");
        assert_eq!(get_header(&hangup, "Action"), Some("Hangup"));
        assert_eq!(get_header(&hangup, "Channel"), Some("PJSIP/100-00000001"));
        let hangup_id = get_header(&hangup, "ActionID").expect("hangup ActionID");
        conn.send_message(&[
            ("Response", "Success"),
            ("ActionID", hangup_id),
            ("Message", "Channel Hungup"),
        ])
        .await;
        while conn.read_message().await.is_some() {}
    });

    let client = connect_client(port, 32).await;
    let pbx = Pbx::new(client.clone());
    let mut variables = HashMap::new();
    variables.insert("TRACE_ID".to_owned(), "trace-7".to_owned());
    variables.insert("ACCOUNT".to_owned(), "alpha".to_owned());
    let mut options = DialOptions::new().caller_id("Test <1234>").timeout_ms(1200);
    options.variables = Some(variables);

    let mut call = pbx
        .dial("PJSIP/100", "200", Some(options))
        .await
        .expect("dial completion");
    assert_eq!(call.channel, "PJSIP/100-00000001");
    assert_eq!(call.unique_id, "call-uid");
    call.wait_for_answer(Duration::from_millis(50))
        .await
        .expect("buffered answer");
    let response = call.hangup().await.expect("hangup completion");
    assert!(response.success);
    assert_eq!(response.message.as_deref(), Some("Channel Hungup"));

    disconnect(&client, handle).await;
}

#[tokio::test]
async fn dial_returns_immediate_originate_rejection_without_waiting_for_an_event() {
    let server = MockAmiServer::start().await;
    let port = server.port();
    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;
        let message = conn.read_message().await.expect("originate action");
        assert_eq!(get_header(&message, "Action"), Some("Originate"));
        assert_eq!(get_header(&message, "Channel"), Some("PJSIP/missing"));
        assert_eq!(get_header(&message, "Exten"), Some("200"));
        assert_eq!(
            get_header(&message, "Timeout"),
            Some(u64::MAX.to_string().as_str())
        );
        let action_id = get_header(&message, "ActionID")
            .expect("ActionID")
            .to_owned();
        conn.send_message(&[
            ("Response", "Error"),
            ("ActionID", &action_id),
            ("Message", "Channel not found"),
        ])
        .await;
        while conn.read_message().await.is_some() {}
    });

    let client = connect_client(port, 32).await;
    let pbx = Pbx::new(client.clone());
    let started = Instant::now();
    let result = pbx
        .dial(
            "PJSIP/missing",
            "200",
            Some(DialOptions::new().timeout_ms(u64::MAX)),
        )
        .await;

    assert!(
        matches!(result, Err(PbxError::CallFailed { cause: 0, ref cause_txt }) if cause_txt == "Channel not found"),
        "unexpected dial result: {result:?}"
    );
    assert!(started.elapsed() < Duration::from_secs(1));
    disconnect(&client, handle).await;
}

#[tokio::test]
async fn dial_returns_matching_originate_event_failure() {
    let server = MockAmiServer::start().await;
    let port = server.port();
    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;
        let (_, action_id) = accept_originate(&mut conn).await;
        send_originate_event(
            &mut conn,
            &action_id,
            "Failure",
            "PJSIP/missing-00000001",
            "failed-uid",
        )
        .await;
        while conn.read_message().await.is_some() {}
    });

    let client = connect_client(port, 32).await;
    let result = Pbx::new(client.clone())
        .dial("PJSIP/missing", "200", None)
        .await;
    assert!(
        matches!(result, Err(PbxError::CallFailed { cause: 0, ref cause_txt }) if cause_txt == "originate failed"),
        "unexpected dial result: {result:?}"
    );
    disconnect(&client, handle).await;
}

#[tokio::test]
async fn wait_for_answer_returns_matching_hangup_failure() {
    let server = MockAmiServer::start().await;
    let port = server.port();
    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;
        let (_, action_id) = accept_originate(&mut conn).await;
        send_originate_event(
            &mut conn,
            &action_id,
            "Success",
            "PJSIP/100-00000002",
            "hangup-uid",
        )
        .await;
        conn.send_message(&[
            ("Event", "Hangup"),
            ("Channel", "PJSIP/other-00000003"),
            ("Uniqueid", "other-uid"),
            ("Cause", "16"),
            ("Cause-txt", "Normal Clearing"),
        ])
        .await;
        conn.send_message(&[
            ("Event", "Hangup"),
            ("Channel", "PJSIP/100-00000002"),
            ("Uniqueid", "hangup-uid"),
            ("Cause", "17"),
            ("Cause-txt", "User busy"),
        ])
        .await;
        while conn.read_message().await.is_some() {}
    });

    let client = connect_client(port, 32).await;
    let mut call = Pbx::new(client.clone())
        .dial("PJSIP/100", "200", None)
        .await
        .expect("dial completion");
    let result = call.wait_for_answer(Duration::from_secs(1)).await;
    assert!(
        matches!(result, Err(PbxError::CallFailed { cause: 17, ref cause_txt }) if cause_txt == "User busy"),
        "unexpected answer result: {result:?}"
    );
    disconnect(&client, handle).await;
}

#[tokio::test]
async fn dial_reports_event_loss_before_originate_completion() {
    let server = MockAmiServer::start().await;
    let port = server.port();
    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;
        let message = conn.read_message().await.expect("originate action");
        let action_id = get_header(&message, "ActionID")
            .expect("ActionID")
            .to_owned();
        for index in 0..3 {
            send_originate_event(
                &mut conn,
                &format!("unrelated-{index}"),
                "Success",
                "PJSIP/unrelated-00000001",
                "unrelated-uid",
            )
            .await;
        }
        conn.send_message(&[
            ("Response", "Success"),
            ("ActionID", &action_id),
            ("Message", "Originate successfully queued"),
        ])
        .await;
        while conn.read_message().await.is_some() {}
    });

    let client = connect_client(port, 1).await;
    let result = Pbx::new(client.clone())
        .dial("PJSIP/100", "200", None)
        .await;
    assert!(
        matches!(result, Err(PbxError::EventLoss { missed }) if missed >= 1),
        "unexpected dial result: {result:?}"
    );
    disconnect(&client, handle).await;
}

#[tokio::test]
async fn wait_for_answer_reports_buffered_event_loss() {
    let server = MockAmiServer::start().await;
    let port = server.port();
    let dial_completed = Arc::new(Notify::new());
    let events_sent = Arc::new(Notify::new());
    let server_dial_completed = dial_completed.clone();
    let server_events_sent = events_sent.clone();
    let handle = server.accept_one(move |mut conn| async move {
        handle_login(&mut conn).await;
        let (_, action_id) = accept_originate(&mut conn).await;
        send_originate_event(
            &mut conn,
            &action_id,
            "Success",
            "PJSIP/100-00000004",
            "lagged-uid",
        )
        .await;
        server_dial_completed.notified().await;
        for index in 0..3 {
            conn.send_message(&[
                ("Event", "Newstate"),
                ("Channel", "PJSIP/other-00000001"),
                ("ChannelState", "4"),
                ("ChannelStateDesc", "Ring"),
                ("Uniqueid", &format!("other-{index}")),
            ])
            .await;
        }
        server_events_sent.notify_one();
        while conn.read_message().await.is_some() {}
    });

    let client = connect_client(port, 2).await;
    let mut call = Pbx::new(client.clone())
        .dial("PJSIP/100", "200", None)
        .await
        .expect("dial completion");
    dial_completed.notify_one();
    events_sent.notified().await;
    let result = call.wait_for_answer(Duration::from_secs(1)).await;
    assert!(
        matches!(result, Err(PbxError::EventLoss { missed }) if missed >= 1),
        "unexpected answer result: {result:?}"
    );
    disconnect(&client, handle).await;
}

#[tokio::test]
async fn wait_for_answer_times_out_without_lifecycle_events() {
    let server = MockAmiServer::start().await;
    let port = server.port();
    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;
        let (_, action_id) = accept_originate(&mut conn).await;
        send_originate_event(
            &mut conn,
            &action_id,
            "Success",
            "PJSIP/100-00000005",
            "timeout-uid",
        )
        .await;
        while conn.read_message().await.is_some() {}
    });

    let client = connect_client(port, 32).await;
    let mut call = Pbx::new(client.clone())
        .dial("PJSIP/100", "200", None)
        .await
        .expect("dial completion");
    let result = call.wait_for_answer(Duration::from_millis(20)).await;
    assert!(matches!(result, Err(PbxError::Timeout)), "{result:?}");
    disconnect(&client, handle).await;
}

#[tokio::test]
async fn dial_times_out_without_originate_completion_event() {
    let server = MockAmiServer::start().await;
    let port = server.port();
    let handle = server.accept_one(|mut conn| async move {
        handle_login(&mut conn).await;
        let _ = accept_originate(&mut conn).await;
        while conn.read_message().await.is_some() {}
    });

    let client = connect_client(port, 32).await;
    let result = Pbx::new(client.clone())
        .dial("PJSIP/100", "200", Some(DialOptions::new().timeout_ms(1)))
        .await;
    assert!(matches!(result, Err(PbxError::Timeout)), "{result:?}");
    disconnect(&client, handle).await;
}
