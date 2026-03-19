#![allow(clippy::unwrap_used)]

// unit tests for AMI call tracker

use std::time::{Duration, Instant};

use asterisk_rs_ami::event::AmiEvent;
use asterisk_rs_ami::tracker::{CallTracker, CompletedCall};
use asterisk_rs_core::event::EventBus;

#[test]
fn test_completed_call_fields() {
    let now = Instant::now();
    let later = now + Duration::from_secs(30);
    let call = CompletedCall {
        channel: "SIP/100-00000001".into(),
        unique_id: "abc.1".into(),
        linked_id: "abc.1".into(),
        start_time: now,
        end_time: later,
        duration: later.duration_since(now),
        cause: 16,
        cause_txt: "Normal Clearing".into(),
        events: vec![],
    };
    assert_eq!(call.channel, "SIP/100-00000001");
    assert_eq!(call.unique_id, "abc.1");
    assert_eq!(call.linked_id, "abc.1");
    assert_eq!(call.cause, 16);
    assert_eq!(call.cause_txt, "Normal Clearing");
    assert_eq!(call.duration, Duration::from_secs(30));
    assert!(call.events.is_empty());
}

#[tokio::test]
async fn test_tracker_processes_call_lifecycle() {
    let bus = EventBus::<AmiEvent>::new(64);
    let sub = bus.subscribe();
    let (tracker, mut rx) = CallTracker::new(sub);

    bus.publish(AmiEvent::NewChannel {
        channel: "SIP/100-00000001".into(),
        channel_state: "0".into(),
        channel_state_desc: "Down".into(),
        caller_id_num: "100".into(),
        caller_id_name: "Test".into(),
        unique_id: "1234.1".into(),
        linked_id: "1234.1".into(),
    });

    bus.publish(AmiEvent::Newstate {
        channel: "SIP/100-00000001".into(),
        channel_state: "6".into(),
        channel_state_desc: "Up".into(),
        unique_id: "1234.1".into(),
    });

    bus.publish(AmiEvent::Hangup {
        channel: "SIP/100-00000001".into(),
        unique_id: "1234.1".into(),
        cause: 16,
        cause_txt: "Normal Clearing".into(),
    });

    // give the background task time to process
    tokio::time::sleep(Duration::from_millis(50)).await;

    let call = rx.recv().await.expect("should receive completed call");
    assert_eq!(call.unique_id, "1234.1");
    assert_eq!(call.channel, "SIP/100-00000001");
    assert_eq!(call.linked_id, "1234.1");
    assert_eq!(call.cause, 16);
    assert_eq!(call.cause_txt, "Normal Clearing");
    // events should include NewChannel + Newstate + Hangup
    assert_eq!(call.events.len(), 3);
    assert!(call.duration >= Duration::ZERO);

    tracker.shutdown();
}
