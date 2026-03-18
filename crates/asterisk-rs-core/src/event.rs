//! Event bus for typed pub/sub within protocol clients.

use std::fmt;
use tokio::sync::broadcast;

/// marker trait for events that can flow through the event bus
pub trait Event: Clone + Send + Sync + fmt::Debug + 'static {}

/// broadcast-based event bus
///
/// each protocol client embeds one of these and publishes events into it.
/// consumers subscribe and receive typed events.
#[derive(Debug)]
pub struct EventBus<E: Event> {
    sender: broadcast::Sender<E>,
}

impl<E: Event> EventBus<E> {
    /// create a new event bus with the given channel capacity
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// publish an event to all subscribers
    ///
    /// returns the number of receivers that got the event.
    /// returns 0 if no subscribers exist (not an error — events are fire-and-forget).
    pub fn publish(&self, event: E) -> usize {
        self.sender.send(event).unwrap_or(0)
    }

    /// subscribe to events
    pub fn subscribe(&self) -> EventSubscription<E> {
        EventSubscription {
            receiver: self.sender.subscribe(),
        }
    }

    /// subscribe with a filter predicate
    ///
    /// only events where `predicate` returns true are delivered
    pub fn subscribe_filtered(
        &self,
        predicate: impl Fn(&E) -> bool + Send + 'static,
    ) -> FilteredSubscription<E> {
        FilteredSubscription {
            inner: self.subscribe(),
            predicate: Box::new(predicate),
        }
    }

    /// number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl<E: Event> Default for EventBus<E> {
    fn default() -> Self {
        Self::new(256)
    }
}

impl<E: Event> Clone for EventBus<E> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

/// subscription handle for receiving events
pub struct EventSubscription<E: Event> {
    receiver: broadcast::Receiver<E>,
}

impl<E: Event> EventSubscription<E> {
    /// receive the next event, waiting until one is available
    ///
    /// returns `None` if the bus is dropped and all buffered events are consumed.
    /// skips over lagged events (slow consumer) with a tracing warning.
    pub async fn recv(&mut self) -> Option<E> {
        loop {
            match self.receiver.recv().await {
                Ok(event) => return Some(event),
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    tracing::warn!(count, "event subscription lagged, dropped events");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    }

    /// add a filter to this subscription, converting it to a filtered subscription
    pub fn with_filter(
        self,
        predicate: impl Fn(&E) -> bool + Send + 'static,
    ) -> FilteredSubscription<E> {
        FilteredSubscription {
            inner: self,
            predicate: Box::new(predicate),
        }
    }
}

/// subscription that filters events before delivering them
pub struct FilteredSubscription<E: Event> {
    inner: EventSubscription<E>,
    predicate: Box<dyn Fn(&E) -> bool + Send>,
}

impl<E: Event> FilteredSubscription<E> {
    /// receive the next event that matches the predicate
    pub async fn recv(&mut self) -> Option<E> {
        loop {
            let event = self.inner.recv().await?;
            if (self.predicate)(&event) {
                return Some(event);
            }
        }
    }
}

impl<E: Event> fmt::Debug for FilteredSubscription<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilteredSubscription")
            .finish_non_exhaustive()
    }
}

impl<E: Event> fmt::Debug for EventSubscription<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventSubscription").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestEvent(String);
    impl Event for TestEvent {}

    #[tokio::test]
    async fn publish_and_receive() {
        let bus = EventBus::new(16);
        let mut sub = bus.subscribe();

        bus.publish(TestEvent("hello".into()));

        let event = sub.recv().await.expect("should receive event");
        assert_eq!(event.0, "hello");
    }

    #[test]
    fn publish_to_zero_subscribers_returns_zero() {
        let bus: EventBus<TestEvent> = EventBus::new(16);
        assert_eq!(bus.publish(TestEvent("nobody".into())), 0);
    }

    #[test]
    fn subscriber_count() {
        let bus: EventBus<TestEvent> = EventBus::new(16);
        assert_eq!(bus.subscriber_count(), 0);

        let _sub1 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);

        let _sub2 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 2);

        drop(_sub1);
        // note: broadcast receiver count doesn't decrease on drop until next send
    }

    #[test]
    fn default_capacity() {
        let bus: EventBus<TestEvent> = EventBus::default();
        // default is 256, just verify it doesn't panic
        bus.publish(TestEvent("test".into()));
    }

    #[tokio::test]
    async fn filtered_subscription_only_matches() {
        let bus = EventBus::new(16);
        let mut filtered = bus.subscribe_filtered(|e: &TestEvent| e.0.starts_with("match"));

        bus.publish(TestEvent("skip-this".into()));
        bus.publish(TestEvent("match-this".into()));
        bus.publish(TestEvent("skip-again".into()));
        bus.publish(TestEvent("match-too".into()));

        let e1 = filtered.recv().await.expect("should get first match");
        assert_eq!(e1.0, "match-this");

        let e2 = filtered.recv().await.expect("should get second match");
        assert_eq!(e2.0, "match-too");
    }

    #[tokio::test]
    async fn subscription_with_filter_conversion() {
        let bus = EventBus::new(16);
        let sub = bus.subscribe();
        let mut filtered = sub.with_filter(|e: &TestEvent| e.0 == "target");

        bus.publish(TestEvent("other".into()));
        bus.publish(TestEvent("target".into()));

        let event = filtered.recv().await.expect("should get target");
        assert_eq!(event.0, "target");
    }
}
