#![allow(clippy::unwrap_used)]

// unit tests for AMI call tracker

use std::time::{Duration, Instant};

use asterisk_rs_ami::event::AmiEvent;
use asterisk_rs_ami::tracker::{CallTracker, CallTrackerConfig, CompletedCall};
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
        events_truncated: 0,
    };
    assert_eq!(call.channel, "SIP/100-00000001");
    assert_eq!(call.unique_id, "abc.1");
    assert_eq!(call.linked_id, "abc.1");
    assert_eq!(call.cause, 16);
    assert_eq!(call.cause_txt, "Normal Clearing");
    assert_eq!(call.duration, Duration::from_secs(30));
    assert!(call.events.is_empty());
}

fn new_channel(unique_id: &str) -> AmiEvent {
    AmiEvent::NewChannel {
        channel: format!("PJSIP/{unique_id}"),
        channel_state: "0".into(),
        channel_state_desc: "Down".into(),
        caller_id_num: "100".into(),
        caller_id_name: "Test".into(),
        unique_id: unique_id.into(),
        linked_id: format!("linked-{unique_id}"),
    }
}

#[test]
fn event_id_accessors_are_canonical() {
    let event = new_channel("uid-1");
    assert_eq!(event.unique_id(), Some("uid-1"));
    assert_eq!(event.linked_id(), Some("linked-uid-1"));
    assert_eq!(event.action_id(), None);

    let originate = AmiEvent::OriginateResponse {
        action_id: Some("action-1".into()),
        channel: "PJSIP/100".into(),
        unique_id: "uid-1".into(),
        response: "Success".into(),
        reason: "4".into(),
    };
    assert_eq!(originate.action_id(), Some("action-1"));
    assert_eq!(originate.unique_id(), Some("uid-1"));
}

#[tokio::test]
async fn tracker_invalidates_and_terminates_after_event_lag() {
    let bus = EventBus::<AmiEvent>::new(1);
    let (tracker, mut rx) = CallTracker::new(bus.subscribe());
    bus.publish(new_channel("lost-1"));
    bus.publish(new_channel("lost-2"));

    tokio::time::timeout(Duration::from_secs(1), async {
        while tracker.stats().valid {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("tracker should observe lag");

    assert_eq!(tracker.stats().lagged_events, 1);
    assert_eq!(tracker.stats().active_calls, 0);
    assert!(rx.recv().await.is_none(), "invalid tracker must terminate");
}

#[tokio::test]
async fn tracker_bounds_active_calls_and_event_history() {
    let bus = EventBus::<AmiEvent>::new(32);
    let config = CallTrackerConfig {
        max_active_calls: 1,
        max_events_per_call: 2,
        ..CallTrackerConfig::default()
    };
    let (tracker, mut rx) = CallTracker::with_config(bus.subscribe(), config);
    bus.publish(new_channel("evicted"));
    bus.publish(new_channel("retained"));
    bus.publish(AmiEvent::Newstate {
        channel: "PJSIP/retained".into(),
        channel_state: "6".into(),
        channel_state_desc: "Up".into(),
        unique_id: "retained".into(),
    });
    bus.publish(AmiEvent::Hangup {
        channel: "PJSIP/retained".into(),
        unique_id: "retained".into(),
        cause: 16,
        cause_txt: "Normal Clearing".into(),
    });

    let call = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("completion timeout")
        .expect("completion channel closed");
    assert_eq!(call.events.len(), 2);
    assert_eq!(call.events_truncated, 1);
    let stats = tracker.stats();
    assert_eq!(stats.active_calls_evicted, 1);
    assert_eq!(stats.truncated_events, 1);
    assert_eq!(stats.active_calls, 0);
}

#[tokio::test]
async fn tracker_periodically_evicts_without_new_events() {
    let bus = EventBus::<AmiEvent>::new(8);
    let config = CallTrackerConfig {
        call_ttl: Duration::from_millis(10),
        eviction_interval: Duration::from_millis(5),
        ..CallTrackerConfig::default()
    };
    let (tracker, mut rx) = CallTracker::with_config(bus.subscribe(), config);
    bus.publish(new_channel("stale"));

    let call = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("periodic eviction timeout")
        .expect("completion channel closed");
    assert_eq!(call.unique_id, "stale");
    assert_eq!(call.cause, 0);
    assert_eq!(tracker.stats().active_calls_evicted, 1);
}

#[tokio::test]
async fn tracker_reports_completed_delivery_loss() {
    let bus = EventBus::<AmiEvent>::new(16);
    let config = CallTrackerConfig {
        completed_capacity: 1,
        ..CallTrackerConfig::default()
    };
    let (tracker, _rx) = CallTracker::with_config(bus.subscribe(), config);
    for id in ["one", "two"] {
        bus.publish(new_channel(id));
        bus.publish(AmiEvent::Hangup {
            channel: format!("PJSIP/{id}"),
            unique_id: id.into(),
            cause: 16,
            cause_txt: "Normal Clearing".into(),
        });
    }

    tokio::time::timeout(Duration::from_secs(1), async {
        while tracker.stats().completed_calls_lost == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("completed loss should become observable");
    assert_eq!(tracker.stats().completed_calls_lost, 1);
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
    let stats = tracker.stats();
    assert!(!stats.valid, "shutdown tracker state must be invalid");
    assert_eq!(stats.active_calls, 0, "shutdown must clear active state");
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

#[tokio::test]
async fn test_tracker_dropped_count_starts_at_zero() {
    let bus = EventBus::<AmiEvent>::new(64);
    let sub = bus.subscribe();
    let (tracker, _rx) = CallTracker::new(sub);

    assert_eq!(tracker.dropped_count(), 0);

    tracker.shutdown();
}
