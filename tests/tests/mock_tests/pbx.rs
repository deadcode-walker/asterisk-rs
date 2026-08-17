use std::time::{Duration, Instant};

use asterisk_rs::pbx::{DialOptions, Pbx, PbxError};
use asterisk_rs_ami::AmiClient;
use asterisk_rs_core::config::ReconnectPolicy;
use asterisk_rs_tests::helpers::assert_server_ok;
use asterisk_rs_tests::mock::ami_server::{MockAmiServer, get_header, handle_login};

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

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(port)
        .credentials("admin", "secret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(2))
        .build()
        .await
        .expect("AMI client");
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

    client.disconnect().await.expect("disconnect");
    assert_server_ok(handle.await);
}
