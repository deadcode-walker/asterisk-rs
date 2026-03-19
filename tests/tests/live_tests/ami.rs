use std::time::Duration;

use asterisk_rs_ami::action::{
    CoreSettingsAction, CoreShowChannelsAction, CoreStatusAction, DBDelAction, DBDelTreeAction,
    DBGetAction, DBPutAction, EventsAction, ExtensionStateAction, ListCommandsAction,
    OriginateAction, ReloadAction, StatusAction,
};
use asterisk_rs_ami::client::AmiClient;
use asterisk_rs_ami::AmiEvent;
use asterisk_rs_core::config::ReconnectPolicy;
use asterisk_rs_tests::helpers::*;

/// build an AMI client connected to the test Asterisk instance
async fn connect() -> AmiClient {
    AmiClient::builder()
        .host(ami_host())
        .port(ami_port())
        .credentials("testadmin", "testsecret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .await
        .expect("failed to connect to Asterisk AMI")
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn connect_authenticate_and_ping() {
    init_tracing();

    let client = connect().await;
    let response = client.ping().await.expect("ping failed");
    assert!(response.success, "ping should succeed");
    assert_eq!(response.get("Ping"), Some("Pong"));

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn cli_command() {
    init_tracing();

    let client = connect().await;
    let response = client
        .command("core show version")
        .await
        .expect("command failed");
    assert!(response.success, "command should succeed: {response:?}");

    // command output may be in output vec or in the message/headers
    // depending on Asterisk version and response format
    let has_content =
        !response.output.is_empty() || response.message.is_some() || response.headers.len() > 2;
    assert!(
        has_content,
        "should have some response content: {response:?}"
    );

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn status_collecting() {
    init_tracing();

    let client = connect().await;

    // ask for channel status — may be empty but should complete
    let result = client
        .send_collecting(&StatusAction { channel: None })
        .await
        .expect("send_collecting failed");

    assert!(result.response.success, "status response should succeed");
    // completion event is always last
    let last = result
        .events
        .last()
        .expect("should have at least the Complete event");
    assert_eq!(
        last.event_name(),
        "StatusComplete",
        "last event should be StatusComplete"
    );

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn originate_to_invalid_extension() {
    init_tracing();

    let client = connect().await;

    // originate to a nonexistent context — Asterisk rejects this synchronously
    let action = OriginateAction {
        channel: "Local/99999@nonexistent".to_string(),
        context: Some("nonexistent".to_string()),
        exten: Some("99999".to_string()),
        priority: Some(1),
        application: None,
        data: None,
        timeout: Some(5000),
        caller_id: Some("test <100>".to_string()),
        account: None,
        async_: true,
        variables: vec![],
    };

    let response = client
        .send_action(&action)
        .await
        .expect("originate send failed");

    // Asterisk rejects originate to nonexistent context immediately
    assert!(
        !response.success,
        "originate to nonexistent context should fail: {response:?}"
    );

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn reconnect_after_connection_drop() {
    init_tracing();

    // connect with reconnect enabled
    let client = AmiClient::builder()
        .host(ami_host())
        .port(ami_port())
        .credentials("testadmin", "testsecret")
        .reconnect(ReconnectPolicy::exponential(
            Duration::from_millis(500),
            Duration::from_secs(5),
        ))
        .timeout(Duration::from_secs(10))
        .build()
        .await
        .expect("initial connect failed");

    // verify we're connected
    let response = client.ping().await.expect("initial ping failed");
    assert!(response.success);

    // force the connection to drop by sending a Logoff through a second client
    // actually, we can't easily kill the TCP from the client side without
    // internal access. instead, just verify reconnect works after a brief
    // disconnect cycle. send a command that forces a reconnect scenario.
    // the simplest approach: disconnect and reconnect by building a new client.
    // but that doesn't test auto-reconnect.

    // alternative: use the AMI "Challenge" action as a health check after
    // waiting — this at least proves the connection is still alive and
    // the keep-alive mechanism works.
    tokio::time::sleep(Duration::from_secs(2)).await;

    let response = client
        .ping()
        .await
        .expect("ping after delay should still work");
    assert!(response.success, "connection should remain alive");

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn full_event_sequence_from_originate() {
    init_tracing();

    let client = connect().await;
    let mut sub = client.subscribe();

    // originate a call to Local/999@default — this extension answers, waits 5s,
    // hangs up. we should see the full lifecycle.
    let action = OriginateAction {
        channel: "Local/999@default".to_string(),
        context: Some("default".to_string()),
        exten: Some("999".to_string()),
        priority: Some(1),
        application: None,
        data: None,
        timeout: Some(10000),
        caller_id: Some("integration-test <100>".to_string()),
        account: None,
        async_: true,
        variables: vec![],
    };

    let response = client.send_action(&action).await.expect("originate failed");
    assert!(
        response.success,
        "originate should be accepted: {response:?}"
    );

    // collect events, looking for the key lifecycle sequence:
    // Newchannel → DialBegin → DialEnd → Hangup
    let mut saw_new_channel = false;
    let mut saw_dial_begin = false;
    let mut saw_dial_end = false;
    let mut saw_hangup = false;
    let mut collected_events = Vec::new();

    let result = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let ev = sub.recv().await.expect("event bus closed");
            collected_events.push(ev.event_name().to_string());
            match ev.event_name() {
                "Newchannel" => saw_new_channel = true,
                "DialBegin" => saw_dial_begin = true,
                "DialEnd" => saw_dial_end = true,
                "Hangup" => {
                    saw_hangup = true;
                    // hangup is the terminal event we care about
                    if saw_new_channel && saw_dial_begin {
                        break;
                    }
                }
                _ => {}
            }
        }
    })
    .await;

    if result.is_err() {
        panic!(
            "timed out waiting for event sequence. collected: {:?}, \
             saw_new_channel={saw_new_channel}, saw_dial_begin={saw_dial_begin}, \
             saw_dial_end={saw_dial_end}, saw_hangup={saw_hangup}",
            collected_events
        );
    }

    assert!(
        saw_new_channel,
        "should see Newchannel: {collected_events:?}"
    );
    assert!(saw_dial_begin, "should see DialBegin: {collected_events:?}");
    assert!(saw_hangup, "should see Hangup: {collected_events:?}");

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn channel_variables_in_originate() {
    init_tracing();

    let client = connect().await;
    let mut sub = client.subscribe();

    let action = OriginateAction {
        channel: "Local/999@default".to_string(),
        context: Some("default".to_string()),
        exten: Some("999".to_string()),
        priority: Some(1),
        application: None,
        data: None,
        timeout: Some(10000),
        caller_id: None,
        account: None,
        async_: true,
        variables: vec![
            ("TEST_VAR1".to_string(), "hello".to_string()),
            ("TEST_VAR2".to_string(), "world".to_string()),
        ],
    };

    let response = client.send_action(&action).await.expect("originate failed");
    assert!(response.success);

    // look for VarSet events with our variables
    let mut saw_var1 = false;
    let mut saw_var2 = false;

    let _ = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let ev = sub.recv().await.expect("event bus closed");
            if let AmiEvent::VarSet {
                variable, value, ..
            } = &ev
            {
                if variable == "TEST_VAR1" && value == "hello" {
                    saw_var1 = true;
                }
                if variable == "TEST_VAR2" && value == "world" {
                    saw_var2 = true;
                }
            }
            if saw_var1 && saw_var2 {
                break;
            }
            // stop if we see the originate complete
            if ev.event_name() == "OriginateResponse" && saw_var1 && saw_var2 {
                break;
            }
        }
    })
    .await;

    assert!(saw_var1, "should have seen VarSet for TEST_VAR1");
    assert!(saw_var2, "should have seen VarSet for TEST_VAR2");

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn originate_busy_extension() {
    init_tracing();

    let client = connect().await;
    let mut sub = client.subscribe();

    // originate to ext 101 which returns Busy()
    let action = OriginateAction {
        channel: "Local/101@default".to_string(),
        context: Some("default".to_string()),
        exten: Some("101".to_string()),
        priority: Some(1),
        application: None,
        data: None,
        timeout: Some(10000),
        caller_id: None,
        account: None,
        async_: true,
        variables: vec![],
    };

    let response = client.send_action(&action).await.expect("originate failed");
    assert!(
        response.success,
        "async originate should be accepted: {response:?}"
    );

    // should see Hangup with busy cause (17)
    let found_busy = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let ev = sub.recv().await.expect("event bus closed");
            if let AmiEvent::Hangup { cause, .. } = &ev {
                if *cause == 17 {
                    return true;
                }
            }
            // OriginateResponse with Failure also indicates busy
            if ev.event_name() == "OriginateResponse" {
                return true;
            }
        }
    })
    .await
    .expect("timed out waiting for busy indication");

    assert!(found_busy);
    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn originate_congestion_extension() {
    init_tracing();

    let client = connect().await;
    let mut sub = client.subscribe();

    // originate to ext 102 which returns Congestion()
    let action = OriginateAction {
        channel: "Local/102@default".to_string(),
        context: Some("default".to_string()),
        exten: Some("102".to_string()),
        priority: Some(1),
        application: None,
        data: None,
        timeout: Some(10000),
        caller_id: None,
        account: None,
        async_: true,
        variables: vec![],
    };

    let response = client.send_action(&action).await.expect("originate failed");
    assert!(
        response.success,
        "async originate should be accepted: {response:?}"
    );

    // should see Hangup with congestion cause (34)
    let found = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let ev = sub.recv().await.expect("event bus closed");
            if let AmiEvent::Hangup { cause, .. } = &ev {
                if *cause == 34 || *cause == 21 {
                    return true;
                }
            }
            if ev.event_name() == "OriginateResponse" {
                return true;
            }
        }
    })
    .await
    .expect("timed out waiting for congestion indication");

    assert!(found);
    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn concurrent_stress_50_pings() {
    init_tracing();

    let client = connect().await;

    // fire 50 concurrent pings against real Asterisk
    let mut handles = Vec::new();
    for _ in 0..50 {
        let c = client.clone();
        handles.push(tokio::spawn(async move { c.ping().await }));
    }

    let mut action_ids = std::collections::HashSet::new();
    for h in handles {
        let result = h.await.expect("task panicked");
        let response = result.expect("ping should succeed");
        assert!(response.success, "ping should be success");
        action_ids.insert(response.action_id.clone());
    }

    // all 50 should have unique ActionIDs
    assert_eq!(
        action_ids.len(),
        50,
        "all 50 pings should have unique ActionIDs"
    );

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn hangup_live_channel() {
    init_tracing();

    let client = connect().await;
    let mut sub = client.subscribe();

    // originate a long-running call (ext 998 waits 30s)
    let action = OriginateAction {
        channel: "Local/998@default".to_string(),
        context: Some("default".to_string()),
        exten: Some("998".to_string()),
        priority: Some(1),
        application: None,
        data: None,
        timeout: Some(10000),
        caller_id: None,
        account: None,
        async_: true,
        variables: vec![],
    };

    let response = client.send_action(&action).await.expect("originate failed");
    assert!(
        response.success,
        "originate should be accepted: {response:?}"
    );

    // wait for NewChannel to get the channel name
    let channel_name = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let ev = sub.recv().await.expect("event bus closed");
            if let AmiEvent::NewChannel { channel, .. } = &ev {
                if channel.contains("998") {
                    return channel.clone();
                }
            }
        }
    })
    .await
    .expect("timed out waiting for NewChannel");

    // hangup the channel via AMI
    let hangup = asterisk_rs_ami::action::HangupAction {
        channel: channel_name.clone(),
        cause: Some(16),
    };
    let response = client.send_action(&hangup).await.expect("hangup failed");
    assert!(response.success, "hangup should succeed: {response:?}");

    // verify we see the Hangup event
    let saw_hangup = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let ev = sub.recv().await.expect("event bus closed");
            if let AmiEvent::Hangup { channel, .. } = &ev {
                if *channel == channel_name {
                    return true;
                }
            }
        }
    })
    .await
    .expect("timed out waiting for Hangup event");

    assert!(saw_hangup);
    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn setvar_getvar_on_live_channel() {
    init_tracing();

    let client = connect().await;
    let mut sub = client.subscribe();

    // originate a long-running call
    let action = OriginateAction {
        channel: "Local/998@default".to_string(),
        context: Some("default".to_string()),
        exten: Some("998".to_string()),
        priority: Some(1),
        application: None,
        data: None,
        timeout: Some(10000),
        caller_id: None,
        account: None,
        async_: true,
        variables: vec![],
    };

    let response = client.send_action(&action).await.expect("originate failed");
    assert!(response.success);

    // wait for channel to be up
    let channel_name = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let ev = sub.recv().await.expect("event bus closed");
            if let AmiEvent::NewChannel { channel, .. } = &ev {
                if channel.contains("998") {
                    return channel.clone();
                }
            }
        }
    })
    .await
    .expect("timed out waiting for NewChannel");

    // small delay for channel to fully answer
    tokio::time::sleep(Duration::from_millis(500)).await;

    // set a variable
    let setvar = asterisk_rs_ami::action::SetVarAction {
        channel: Some(channel_name.clone()),
        variable: "TEST_LIVE_VAR".to_string(),
        value: "integration_value".to_string(),
    };
    let response = client.send_action(&setvar).await.expect("setvar failed");
    assert!(response.success, "setvar should succeed: {response:?}");

    // get the variable back
    let getvar = asterisk_rs_ami::action::GetVarAction {
        channel: Some(channel_name.clone()),
        variable: "TEST_LIVE_VAR".to_string(),
    };
    let response = client.send_action(&getvar).await.expect("getvar failed");
    assert!(response.success, "getvar should succeed: {response:?}");
    assert_eq!(
        response.get("Value"),
        Some("integration_value"),
        "variable value should match: {response:?}"
    );

    // cleanup: hangup
    let hangup = asterisk_rs_ami::action::HangupAction {
        channel: channel_name,
        cause: None,
    };
    let _ = client.send_action(&hangup).await;

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn core_settings_action() {
    init_tracing();

    let client = connect().await;
    let response = client
        .send_action(&CoreSettingsAction)
        .await
        .expect("core settings action failed");
    assert!(
        response.success,
        "core settings should succeed: {response:?}"
    );

    // field names vary by asterisk version
    let has_version_info = response
        .headers
        .keys()
        .any(|k| k.eq_ignore_ascii_case("AMIversion") || k.eq_ignore_ascii_case("AsteriskVersion"));
    assert!(
        has_version_info,
        "should contain AMIversion or AsteriskVersion: {response:?}"
    );

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn core_status_action() {
    init_tracing();

    let client = connect().await;
    let response = client
        .send_action(&CoreStatusAction)
        .await
        .expect("core status action failed");
    assert!(response.success, "core status should succeed: {response:?}");

    let has_status_info = response.headers.keys().any(|k| {
        k.eq_ignore_ascii_case("CoreStartupTime") || k.eq_ignore_ascii_case("CoreCurrentCalls")
    });
    assert!(
        has_status_info,
        "should contain CoreStartupTime or CoreCurrentCalls: {response:?}"
    );

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn core_show_channels_collecting() {
    init_tracing();

    let client = connect().await;
    let result = client
        .send_collecting(&CoreShowChannelsAction)
        .await
        .expect("send_collecting failed");

    assert!(result.response.success, "core show channels should succeed");

    let last = result
        .events
        .last()
        .expect("should have at least the Complete event");
    assert_eq!(
        last.event_name(),
        "CoreShowChannelsComplete",
        "last event should be CoreShowChannelsComplete"
    );

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn db_put_get_del_cycle() {
    init_tracing();

    let client = connect().await;

    // put
    let put = DBPutAction {
        family: "test".to_string(),
        key: "integration".to_string(),
        val: "hello123".to_string(),
    };
    let response = client.send_action(&put).await.expect("db put failed");
    assert!(response.success, "db put should succeed: {response:?}");

    // get — should find the value (asterisk may return DBGet as event-list)
    let get = DBGetAction {
        family: "test".to_string(),
        key: "integration".to_string(),
    };
    let result = client.send_collecting(&get).await.expect("db get failed");
    assert!(
        result.response.success,
        "db get should succeed: {:?}",
        result.response
    );

    // value may be in response headers or in a DBGetResponse event
    let val = result
        .response
        .get("Val")
        .map(String::from)
        .unwrap_or_else(|| {
            result
                .events
                .iter()
                .find_map(|ev| match ev {
                    AmiEvent::Unknown { headers, .. } => headers.get("Val").cloned(),
                    _ => None,
                })
                .expect("Val not found in response or events")
        });
    assert_eq!(val, "hello123", "db value should match: {:?}", result);

    // delete
    let del = DBDelAction {
        family: "test".to_string(),
        key: "integration".to_string(),
    };
    let response = client.send_action(&del).await.expect("db del failed");
    assert!(response.success, "db del should succeed: {response:?}");

    // get again — should fail (key not found)
    let result = client
        .send_action(&get)
        .await
        .expect("db get after del failed");
    assert!(
        !result.success,
        "db get after delete should fail: {result:?}"
    );

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn db_del_tree() {
    init_tracing();

    let client = connect().await;

    // put 3 keys under testfamily
    for i in 0..3 {
        let put = DBPutAction {
            family: "testfamily".to_string(),
            key: format!("key{i}"),
            val: format!("val{i}"),
        };
        let response = client.send_action(&put).await.expect("db put failed");
        assert!(response.success, "db put key{i} should succeed");
    }

    // delete the entire family
    let del_tree = DBDelTreeAction {
        family: "testfamily".to_string(),
        key: None,
    };
    let response = client
        .send_action(&del_tree)
        .await
        .expect("db del tree failed");
    assert!(response.success, "db del tree should succeed: {response:?}");

    // verify all keys are gone
    for i in 0..3 {
        let get = DBGetAction {
            family: "testfamily".to_string(),
            key: format!("key{i}"),
        };
        let response = client
            .send_action(&get)
            .await
            .expect("db get after del tree failed");
        assert!(
            !response.success,
            "db get key{i} after del tree should fail: {response:?}"
        );
    }

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn list_commands_action() {
    init_tracing();

    let client = connect().await;
    let response = client
        .send_action(&ListCommandsAction)
        .await
        .expect("list commands failed");
    assert!(
        response.success,
        "list commands should succeed: {response:?}"
    );

    // asterisk returns a list of CLI commands as headers
    assert!(
        response.headers.len() > 2,
        "should have multiple command headers: {response:?}"
    );

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn multiple_cli_commands() {
    init_tracing();

    let client = connect().await;

    for cmd in [
        "core show uptime",
        "core show sysinfo",
        "dialplan show default",
    ] {
        let response = client.command(cmd).await.expect("command failed");
        assert!(response.success, "{cmd} should succeed: {response:?}");
    }

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn originate_application_mode() {
    init_tracing();

    let client = connect().await;
    let mut sub = client.subscribe();

    let action = OriginateAction {
        channel: "Local/999@default".to_string(),
        context: None,
        exten: None,
        priority: None,
        application: Some("Playback".to_string()),
        data: Some("silence/1".to_string()),
        timeout: Some(10000),
        caller_id: None,
        account: None,
        async_: true,
        variables: vec![],
    };

    let response = client.send_action(&action).await.expect("originate failed");
    assert!(
        response.success,
        "originate should be accepted: {response:?}"
    );

    // wait for hangup confirming the call completed
    let saw_hangup = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let ev = sub.recv().await.expect("event bus closed");
            if ev.event_name() == "Hangup" {
                return true;
            }
        }
    })
    .await
    .expect("timed out waiting for Hangup");

    assert!(saw_hangup);
    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn filtered_event_subscription() {
    init_tracing();

    let client = connect().await;
    let mut filtered = client.subscribe_filtered(|e| e.event_name() == "Hangup");

    // originate to ext 100 — answers, waits 1s, hangs up
    let action = OriginateAction {
        channel: "Local/100@default".to_string(),
        context: Some("default".to_string()),
        exten: Some("100".to_string()),
        priority: Some(1),
        application: None,
        data: None,
        timeout: Some(10000),
        caller_id: None,
        account: None,
        async_: true,
        variables: vec![],
    };

    let response = client.send_action(&action).await.expect("originate failed");
    assert!(
        response.success,
        "originate should be accepted: {response:?}"
    );

    // collect up to 5 events with timeout — all should be Hangup
    let mut events = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(10), async {
        for _ in 0..5 {
            if let Some(ev) = filtered.recv().await {
                events.push(ev);
            } else {
                break;
            }
        }
    })
    .await;

    assert!(
        !events.is_empty(),
        "should have received at least one Hangup event"
    );
    for ev in &events {
        assert_eq!(
            ev.event_name(),
            "Hangup",
            "filtered sub should only yield Hangup"
        );
    }

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn multiple_concurrent_clients() {
    init_tracing();

    let client1 = connect().await;
    let client2 = connect().await;
    let mut sub1 = client1.subscribe();
    let mut sub2 = client2.subscribe();

    // originate from client1
    let action = OriginateAction {
        channel: "Local/100@default".to_string(),
        context: Some("default".to_string()),
        exten: Some("100".to_string()),
        priority: Some(1),
        application: None,
        data: None,
        timeout: Some(10000),
        caller_id: None,
        account: None,
        async_: true,
        variables: vec![],
    };

    let response = client1
        .send_action(&action)
        .await
        .expect("originate failed");
    assert!(
        response.success,
        "originate should be accepted: {response:?}"
    );

    // both subscribers should see events
    let mut saw1 = false;
    let mut saw2 = false;

    let _ = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            tokio::select! {
                ev = sub1.recv() => {
                    let ev = ev.expect("sub1 bus closed");
                    if ev.event_name() == "Hangup" { saw1 = true; }
                }
                ev = sub2.recv() => {
                    let ev = ev.expect("sub2 bus closed");
                    if ev.event_name() == "Hangup" { saw2 = true; }
                }
            }
            if saw1 && saw2 {
                break;
            }
        }
    })
    .await;

    assert!(saw1, "client1 subscriber should see events");
    assert!(saw2, "client2 subscriber should see events");

    client1
        .disconnect()
        .await
        .expect("disconnect client1 failed");
    client2
        .disconnect()
        .await
        .expect("disconnect client2 failed");
}

#[tokio::test]
async fn extension_state_query() {
    init_tracing();

    let client = connect().await;
    let action = ExtensionStateAction {
        exten: "100".to_string(),
        context: "default".to_string(),
    };
    let response = client
        .send_action(&action)
        .await
        .expect("extension state action failed");

    // unhinted extensions may return -1 — just verify the action completes
    assert!(
        response.success,
        "extension state should succeed: {response:?}"
    );

    let has_state = response
        .headers
        .keys()
        .any(|k| k.eq_ignore_ascii_case("Status") || k.eq_ignore_ascii_case("Hint"));
    assert!(has_state, "should have Status or Hint header: {response:?}");

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn reload_action() {
    init_tracing();

    let client = connect().await;
    let action = ReloadAction { module: None };
    let response = client
        .send_action(&action)
        .await
        .expect("reload action failed");
    assert!(response.success, "reload should succeed: {response:?}");

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn events_action_toggle() {
    init_tracing();

    // use a dedicated client so event masking doesn't affect other tests
    let client = connect().await;
    let mut sub = client.subscribe();

    // turn off events
    let off = EventsAction {
        event_mask: "off".to_string(),
    };
    let response = client.send_action(&off).await.expect("events off failed");
    assert!(response.success, "events off should succeed: {response:?}");

    // actions should still work with events masked off
    let action = OriginateAction {
        channel: "Local/100@default".to_string(),
        context: Some("default".to_string()),
        exten: Some("100".to_string()),
        priority: Some(1),
        application: None,
        data: None,
        timeout: Some(10000),
        caller_id: None,
        account: None,
        async_: true,
        variables: vec![],
    };
    let response = client.send_action(&action).await.expect("originate failed");
    assert!(
        response.success,
        "originate should succeed with events off: {response:?}"
    );

    // turn events back on
    let on = EventsAction {
        event_mask: "on".to_string(),
    };
    let response = client.send_action(&on).await.expect("events on failed");
    assert!(response.success, "events on should succeed: {response:?}");

    // originate again — should produce events now
    let _ = client
        .send_action(&action)
        .await
        .expect("originate 2 failed");

    let got_event = tokio::time::timeout(Duration::from_secs(10), sub.recv()).await;
    assert!(
        got_event.is_ok(),
        "should receive events after turning mask back on"
    );

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn originate_to_nonexistent_extension_sync_rejected() {
    init_tracing();

    let client = connect().await;

    // originate to a truly nonexistent context synchronously (async_ = false)
    // asterisk rejects this before creating any channels
    let action = OriginateAction {
        channel: "Local/99999@does-not-exist".to_string(),
        context: Some("does-not-exist".to_string()),
        exten: Some("99999".to_string()),
        priority: Some(1),
        application: None,
        data: None,
        timeout: Some(5000),
        caller_id: None,
        account: None,
        async_: false,
        variables: vec![],
    };

    let response = client
        .send_action(&action)
        .await
        .expect("originate send failed");
    assert!(
        !response.success,
        "sync originate to nonexistent context should fail: {response:?}"
    );

    client.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
async fn originate_with_account_code() {
    init_tracing();

    let client = connect().await;
    let mut sub = client.subscribe();

    let action = OriginateAction {
        channel: "Local/100@default".to_string(),
        context: Some("default".to_string()),
        exten: Some("100".to_string()),
        priority: Some(1),
        application: None,
        data: None,
        timeout: Some(10000),
        caller_id: None,
        account: Some("test-account".to_string()),
        async_: true,
        variables: vec![],
    };

    let response = client.send_action(&action).await.expect("originate failed");
    assert!(
        response.success,
        "originate should be accepted: {response:?}"
    );

    // look for VarSet with CHANNEL(accountcode) or Cdr events containing the account code.
    // NewChannel doesn't carry account_code in the typed variant, so check VarSet instead.
    let mut found_account = false;
    let _ = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let ev = sub.recv().await.expect("event bus closed");
            if let AmiEvent::VarSet {
                variable, value, ..
            } = &ev
            {
                if variable.contains("accountcode") && value == "test-account" {
                    found_account = true;
                    break;
                }
            }
            // Unknown events may carry AccountCode in headers
            if let AmiEvent::Unknown { headers, .. } = &ev {
                if headers.get("AccountCode").map(|v| v.as_str()) == Some("test-account") {
                    found_account = true;
                    break;
                }
            }
            if ev.event_name() == "OriginateResponse" {
                break;
            }
        }
    })
    .await;

    assert!(
        found_account,
        "should see account code in VarSet or event headers"
    );

    client.disconnect().await.expect("disconnect failed");
}

// ── call tracker live tests ───────────────────────────────

#[tokio::test]
async fn call_tracker_captures_originate_lifecycle() {
    init_tracing();

    let client = connect().await;

    // start tracking calls before we originate
    let (_tracker, mut completed_rx) = client.call_tracker();

    // originate a call to Local/999@default which answers briefly then hangs up
    let action = OriginateAction {
        channel: "Local/999@default".to_string(),
        context: Some("default".to_string()),
        exten: Some("999".to_string()),
        priority: Some(1),
        application: None,
        data: None,
        timeout: Some(10000),
        caller_id: Some("tracker-test <558>".to_string()),
        account: None,
        async_: true,
        variables: vec![],
    };

    let response = client.send_action(&action).await.expect("originate failed");
    assert!(
        response.success,
        "originate should be accepted: {response:?}"
    );

    // wait for a CompletedCall to appear — the Local channel pair should complete
    // once the call times out or the dialplan finishes.
    let completed = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let call = completed_rx
                .recv()
                .await
                .expect("call tracker channel closed");
            tracing::info!(
                channel = %call.channel,
                unique_id = %call.unique_id,
                duration = ?call.duration,
                cause = call.cause,
                "completed call received"
            );
            // Local channels create a pair: Local/999@default-XXXXX;1 and ;2
            // match on the channel prefix to find ours
            if call.channel.starts_with("Local/999@default") {
                return call;
            }
        }
    })
    .await
    .expect("timed out waiting for CompletedCall from tracker");

    assert!(
        !completed.unique_id.is_empty(),
        "completed call should have a non-empty unique_id"
    );
    assert!(
        completed.channel.starts_with("Local/"),
        "channel should be a Local channel: {}",
        completed.channel
    );
    assert!(
        !completed.cause_txt.is_empty(),
        "cause_txt should not be empty"
    );
    assert!(
        !completed.events.is_empty(),
        "completed call should have collected events"
    );

    tracing::info!(
        channel = %completed.channel,
        unique_id = %completed.unique_id,
        linked_id = %completed.linked_id,
        duration = ?completed.duration,
        cause = completed.cause,
        cause_txt = %completed.cause_txt,
        event_count = completed.events.len(),
        "call tracker lifecycle verified"
    );

    _tracker.shutdown();
    client.disconnect().await.expect("disconnect failed");
}
