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
}

impl<E: Event> fmt::Debug for EventSubscription<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventSubscription").finish_non_exhaustive()
    }
}
