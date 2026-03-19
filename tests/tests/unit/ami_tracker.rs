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

// ── additional tracker unit tests ─────────────────────────

#[tokio::test]
async fn test_tracker_multiple_simultaneous_calls() {
    let bus = EventBus::<AmiEvent>::new(64);
    let sub = bus.subscribe();
    let (tracker, mut rx) = CallTracker::new(sub);

    bus.publish(AmiEvent::NewChannel {
        channel: "SIP/100-00000001".into(),
        channel_state: "0".into(),
        channel_state_desc: "Down".into(),
        caller_id_num: "100".into(),
        caller_id_name: "Alice".into(),
        unique_id: "uid-1".into(),
        linked_id: "uid-1".into(),
    });

    bus.publish(AmiEvent::NewChannel {
        channel: "SIP/200-00000002".into(),
        channel_state: "0".into(),
        channel_state_desc: "Down".into(),
        caller_id_num: "200".into(),
        caller_id_name: "Bob".into(),
        unique_id: "uid-2".into(),
        linked_id: "uid-2".into(),
    });

    bus.publish(AmiEvent::Hangup {
        channel: "SIP/100-00000001".into(),
        unique_id: "uid-1".into(),
        cause: 16,
        cause_txt: "Normal Clearing".into(),
    });

    bus.publish(AmiEvent::Hangup {
        channel: "SIP/200-00000002".into(),
        unique_id: "uid-2".into(),
        cause: 17,
        cause_txt: "User Busy".into(),
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let call1 = rx
        .recv()
        .await
        .expect("should receive first completed call");
    let call2 = rx
        .recv()
        .await
        .expect("should receive second completed call");

    let mut uids = vec![call1.unique_id.clone(), call2.unique_id.clone()];
    uids.sort();
    assert_eq!(uids, vec!["uid-1", "uid-2"]);

    tracker.shutdown();
}

#[tokio::test]
async fn test_tracker_hangup_unknown_call_ignored() {
    let bus = EventBus::<AmiEvent>::new(64);
    let sub = bus.subscribe();
    let (tracker, mut rx) = CallTracker::new(sub);

    // hangup without a preceding NewChannel
    bus.publish(AmiEvent::Hangup {
        channel: "SIP/ghost-00000001".into(),
        unique_id: "unknown-uid".into(),
        cause: 16,
        cause_txt: "Normal Clearing".into(),
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // channel should be empty — no CompletedCall produced
    assert!(
        rx.try_recv().is_err(),
        "hangup for unknown call should not produce a CompletedCall"
    );

    tracker.shutdown();
}

#[tokio::test]
async fn test_tracker_events_collected_in_order() {
    let bus = EventBus::<AmiEvent>::new(64);
    let sub = bus.subscribe();
    let (tracker, mut rx) = CallTracker::new(sub);

    bus.publish(AmiEvent::NewChannel {
        channel: "SIP/300-00000003".into(),
        channel_state: "0".into(),
        channel_state_desc: "Down".into(),
        caller_id_num: "300".into(),
        caller_id_name: "Charlie".into(),
        unique_id: "ordered-1".into(),
        linked_id: "ordered-1".into(),
    });

    for state in ["4", "5", "6"] {
        bus.publish(AmiEvent::Newstate {
            channel: "SIP/300-00000003".into(),
            channel_state: state.into(),
            channel_state_desc: "Ringing".into(),
            unique_id: "ordered-1".into(),
        });
    }

    bus.publish(AmiEvent::Hangup {
        channel: "SIP/300-00000003".into(),
        unique_id: "ordered-1".into(),
        cause: 16,
        cause_txt: "Normal Clearing".into(),
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let call = rx.recv().await.expect("should receive completed call");
    assert_eq!(
        call.events.len(),
        5,
        "should have NewChannel + 3 Newstate + Hangup"
    );

    // verify ordering: first is NewChannel, last is Hangup
    assert!(
        matches!(call.events[0], AmiEvent::NewChannel { .. }),
        "first event should be NewChannel"
    );
    assert!(
        matches!(call.events[1], AmiEvent::Newstate { .. }),
        "second event should be Newstate"
    );
    assert!(
        matches!(call.events[4], AmiEvent::Hangup { .. }),
        "last event should be Hangup"
    );

    tracker.shutdown();
}

#[tokio::test]
async fn test_tracker_shutdown_stops_processing() {
    let bus = EventBus::<AmiEvent>::new(64);
    let sub = bus.subscribe();
    let (tracker, mut rx) = CallTracker::new(sub);

    // shutdown immediately before any events
    tracker.shutdown();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // publish events after shutdown
    bus.publish(AmiEvent::NewChannel {
        channel: "SIP/400-00000004".into(),
        channel_state: "0".into(),
        channel_state_desc: "Down".into(),
        caller_id_num: "400".into(),
        caller_id_name: "Dave".into(),
        unique_id: "post-shutdown-1".into(),
        linked_id: "post-shutdown-1".into(),
    });

    bus.publish(AmiEvent::Hangup {
        channel: "SIP/400-00000004".into(),
        unique_id: "post-shutdown-1".into(),
        cause: 16,
        cause_txt: "Normal Clearing".into(),
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(
        rx.try_recv().is_err(),
        "no CompletedCall should be produced after shutdown"
    );
}
