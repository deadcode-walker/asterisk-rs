//! Bounded, loss-aware AMI call correlation.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use asterisk_rs_core::event::{EventReceive, EventSubscription};
use tokio::sync::{mpsc, watch};

use crate::event::AmiEvent;

/// A fully resolved call with its bounded event history.
#[derive(Debug, Clone)]
pub struct CompletedCall {
    /// Last observed channel name.
    pub channel: String,
    /// Asterisk unique identifier for the call leg.
    pub unique_id: String,
    /// Identifier correlating related call legs.
    pub linked_id: String,
    /// Local instant when tracking began.
    pub start_time: Instant,
    /// Local instant when the call completed.
    pub end_time: Instant,
    /// Elapsed tracked lifetime.
    pub duration: Duration,
    /// Numeric Asterisk hangup cause.
    pub cause: u32,
    /// Text supplied with the hangup cause.
    pub cause_txt: String,
    /// Retained events, in arrival order.
    pub events: Vec<AmiEvent>,
    /// Events omitted after the per-call history limit was reached.
    pub events_truncated: u64,
}

/// Resource limits for a [`CallTracker`].
#[derive(Debug, Clone)]
pub struct CallTrackerConfig {
    /// Maximum idle lifetime of an active call.
    pub call_ttl: Duration,
    /// Interval between time-driven eviction sweeps.
    pub eviction_interval: Duration,
    /// Maximum calls retained concurrently.
    pub max_active_calls: usize,
    /// Maximum events retained for one call.
    pub max_events_per_call: usize,
    /// Capacity of completed-call delivery.
    pub completed_capacity: usize,
}

impl Default for CallTrackerConfig {
    fn default() -> Self {
        Self {
            call_ttl: Duration::from_secs(3600),
            eviction_interval: Duration::from_secs(30),
            max_active_calls: 4096,
            max_events_per_call: 256,
            completed_capacity: 256,
        }
    }
}

impl CallTrackerConfig {
    fn normalized(mut self) -> Self {
        self.eviction_interval = self.eviction_interval.max(Duration::from_millis(1));
        self.max_active_calls = self.max_active_calls.max(1);
        self.max_events_per_call = self.max_events_per_call.max(1);
        self.completed_capacity = self.completed_capacity.max(1);
        self
    }
}

/// Snapshot of tracker health and bounded-loss counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallTrackerStats {
    /// Calls currently retained.
    pub active_calls: usize,
    /// Source events missed because the subscription lagged.
    pub lagged_events: u64,
    /// Per-call history events omitted at the configured bound.
    pub truncated_events: u64,
    /// Active calls evicted to preserve the configured bound.
    pub active_calls_evicted: u64,
    /// Completed calls dropped because delivery was full.
    pub completed_calls_lost: u64,
    /// False after source lag or source closure makes tracked state incomplete.
    pub valid: bool,
}

#[derive(Default)]
struct TrackerMetrics {
    active_calls: AtomicUsize,
    lagged_events: AtomicU64,
    truncated_events: AtomicU64,
    active_calls_evicted: AtomicU64,
    completed_calls_lost: AtomicU64,
    valid: AtomicBool,
}

struct ActiveCall {
    channel: String,
    unique_id: String,
    linked_id: String,
    start_time: Instant,
    events: Vec<AmiEvent>,
    events_truncated: u64,
}

/// Correlates AMI events by UniqueID into bounded completed-call records.
pub struct CallTracker {
    shutdown_tx: watch::Sender<bool>,
    task_handle: tokio::task::JoinHandle<()>,
    metrics: Arc<TrackerMetrics>,
}

impl std::fmt::Debug for CallTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallTracker")
            .field("stats", &self.stats())
            .finish_non_exhaustive()
    }
}

impl CallTracker {
    /// Create a tracker with conservative defaults.
    pub fn new(subscription: EventSubscription<AmiEvent>) -> (Self, mpsc::Receiver<CompletedCall>) {
        Self::with_config(subscription, CallTrackerConfig::default())
    }

    /// Create a tracker with explicit resource limits.
    pub fn with_config(
        subscription: EventSubscription<AmiEvent>,
        config: CallTrackerConfig,
    ) -> (Self, mpsc::Receiver<CompletedCall>) {
        let config = config.normalized();
        let (completed_tx, completed_rx) = mpsc::channel(config.completed_capacity);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let metrics = Arc::new(TrackerMetrics::default());
        metrics.valid.store(true, Ordering::Relaxed);
        let task_handle = tokio::spawn(track_loop(
            subscription,
            completed_tx,
            shutdown_rx,
            config,
            Arc::clone(&metrics),
        ));
        (
            Self {
                shutdown_tx,
                task_handle,
                metrics,
            },
            completed_rx,
        )
    }

    /// Current tracker state and cumulative loss counters.
    pub fn stats(&self) -> CallTrackerStats {
        CallTrackerStats {
            active_calls: self.metrics.active_calls.load(Ordering::Relaxed),
            lagged_events: self.metrics.lagged_events.load(Ordering::Relaxed),
            truncated_events: self.metrics.truncated_events.load(Ordering::Relaxed),
            active_calls_evicted: self.metrics.active_calls_evicted.load(Ordering::Relaxed),
            completed_calls_lost: self.metrics.completed_calls_lost.load(Ordering::Relaxed),
            valid: self.metrics.valid.load(Ordering::Relaxed),
        }
    }

    /// Compatibility accessor for completed delivery loss.
    pub fn dropped_count(&self) -> u64 {
        self.stats().completed_calls_lost
    }

    /// Stop tracking immediately and invalidate current state.
    pub fn shutdown(&self) {
        self.metrics.valid.store(false, Ordering::Relaxed);
        self.metrics.active_calls.store(0, Ordering::Relaxed);
        let _ = self.shutdown_tx.send(true);
        self.task_handle.abort();
    }
}

impl Drop for CallTracker {
    fn drop(&mut self) {
        self.shutdown();
    }
}

async fn track_loop(
    mut subscription: EventSubscription<AmiEvent>,
    completed_tx: mpsc::Sender<CompletedCall>,
    mut shutdown_rx: watch::Receiver<bool>,
    config: CallTrackerConfig,
    metrics: Arc<TrackerMetrics>,
) {
    let mut active = HashMap::new();
    let mut eviction = tokio::time::interval(config.eviction_interval);
    eviction.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    eviction.tick().await;

    loop {
        tokio::select! {
            outcome = subscription.recv_outcome() => match outcome {
                EventReceive::Event(event) => handle_event(
                    &mut active, &completed_tx, event, &config, &metrics
                ),
                EventReceive::Lagged(count) => {
                    metrics.lagged_events.fetch_add(count, Ordering::Relaxed);
                    metrics.valid.store(false, Ordering::Relaxed);
                    active.clear();
                    metrics.active_calls.store(0, Ordering::Relaxed);
                    break;
                }
                EventReceive::Closed => {
                    metrics.valid.store(false, Ordering::Relaxed);
                    active.clear();
                    metrics.active_calls.store(0, Ordering::Relaxed);
                    break;
                }
            },
            _ = eviction.tick() => evict_stale(
                &mut active, &completed_tx, config.call_ttl, &metrics
            ),
            changed = shutdown_rx.changed() => {
                if changed.is_err() || *shutdown_rx.borrow() { break; }
            }
        }
    }
}

fn handle_event(
    active: &mut HashMap<String, ActiveCall>,
    completed_tx: &mpsc::Sender<CompletedCall>,
    event: AmiEvent,
    config: &CallTrackerConfig,
    metrics: &TrackerMetrics,
) {
    if let AmiEvent::NewChannel {
        channel,
        unique_id,
        linked_id,
        ..
    } = &event
    {
        if !active.contains_key(unique_id) && active.len() >= config.max_active_calls {
            if let Some(oldest) = active
                .iter()
                .min_by_key(|(_, call)| call.start_time)
                .map(|(id, _)| id.clone())
            {
                active.remove(&oldest);
                metrics.active_calls_evicted.fetch_add(1, Ordering::Relaxed);
            }
        }
        active.insert(
            unique_id.clone(),
            ActiveCall {
                channel: channel.clone(),
                unique_id: unique_id.clone(),
                linked_id: linked_id.clone(),
                start_time: Instant::now(),
                events: vec![event],
                events_truncated: 0,
            },
        );
        metrics.active_calls.store(active.len(), Ordering::Relaxed);
        return;
    }

    let Some(unique_id) = event.unique_id().map(str::to_owned) else {
        return;
    };

    if let AmiEvent::Hangup {
        cause, cause_txt, ..
    } = &event
    {
        let cause = *cause;
        let cause_txt = cause_txt.clone();
        if let Some(mut call) = active.remove(&unique_id) {
            append_event(&mut call, event, config.max_events_per_call, metrics);
            let end_time = Instant::now();
            let completed = CompletedCall {
                channel: call.channel,
                unique_id: call.unique_id,
                linked_id: call.linked_id,
                start_time: call.start_time,
                end_time,
                duration: end_time.duration_since(call.start_time),
                cause,
                cause_txt,
                events: call.events,
                events_truncated: call.events_truncated,
            };
            send_completed(completed_tx, completed, metrics);
            metrics.active_calls.store(active.len(), Ordering::Relaxed);
        }
        return;
    }

    if let Some(call) = active.get_mut(&unique_id) {
        if let AmiEvent::Rename { new_name, .. } = &event {
            call.channel.clone_from(new_name);
        }
        append_event(call, event, config.max_events_per_call, metrics);
    }
}

fn append_event(
    call: &mut ActiveCall,
    event: AmiEvent,
    max_events: usize,
    metrics: &TrackerMetrics,
) {
    if call.events.len() < max_events {
        call.events.push(event);
    } else {
        call.events_truncated = call.events_truncated.saturating_add(1);
        metrics.truncated_events.fetch_add(1, Ordering::Relaxed);
    }
}

fn evict_stale(
    active: &mut HashMap<String, ActiveCall>,
    completed_tx: &mpsc::Sender<CompletedCall>,
    ttl: Duration,
    metrics: &TrackerMetrics,
) {
    let now = Instant::now();
    let stale: Vec<_> = active
        .iter()
        .filter(|(_, call)| now.duration_since(call.start_time) > ttl)
        .map(|(id, _)| id.clone())
        .collect();
    for id in stale {
        let Some(call) = active.remove(&id) else {
            continue;
        };
        metrics.active_calls_evicted.fetch_add(1, Ordering::Relaxed);
        send_completed(
            completed_tx,
            CompletedCall {
                channel: call.channel,
                unique_id: call.unique_id,
                linked_id: call.linked_id,
                start_time: call.start_time,
                end_time: now,
                duration: now.duration_since(call.start_time),
                cause: 0,
                cause_txt: "ttl eviction: no hangup received".into(),
                events: call.events,
                events_truncated: call.events_truncated,
            },
            metrics,
        );
    }
    metrics.active_calls.store(active.len(), Ordering::Relaxed);
}

fn send_completed(
    tx: &mpsc::Sender<CompletedCall>,
    completed: CompletedCall,
    metrics: &TrackerMetrics,
) {
    if tx.try_send(completed).is_err() {
        metrics.completed_calls_lost.fetch_add(1, Ordering::Relaxed);
    }
}
