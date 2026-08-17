//! Event bus for typed pub/sub within protocol clients.

use std::fmt;
use tokio::sync::broadcast;

/// Outcome of receiving from an [`EventSubscription`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventReceive<E> {
    /// The next event was received.
    Event(E),
    /// The subscriber fell behind and this many events were lost.
    Lagged(u64),
    /// The event bus was closed after all buffered events were consumed.
    Closed,
}

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
        // broadcast::channel panics on 0 capacity; clamp to 1 as a safe floor
        let capacity = capacity.max(1);
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
    /// Receive one event-bus outcome without hiding loss.
    pub async fn recv_outcome(&mut self) -> EventReceive<E> {
        match self.receiver.recv().await {
            Ok(event) => EventReceive::Event(event),
            Err(broadcast::error::RecvError::Lagged(count)) => EventReceive::Lagged(count),
            Err(broadcast::error::RecvError::Closed) => EventReceive::Closed,
        }
    }

    /// Receive the next available event while explicitly accepting event loss.
    pub async fn recv_lossy(&mut self) -> Option<E> {
        loop {
            match self.recv_outcome().await {
                EventReceive::Event(event) => return Some(event),
                EventReceive::Lagged(count) => {
                    tracing::warn!(count, "lossy event subscription dropped events");
                }
                EventReceive::Closed => return None,
            }
        }
    }

    /// Receive one event-bus outcome without hiding loss.
    pub async fn recv(&mut self) -> EventReceive<E> {
        self.recv_outcome().await
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
    /// Receive one filtered outcome without hiding loss.
    pub async fn recv_outcome(&mut self) -> EventReceive<E> {
        loop {
            match self.inner.recv_outcome().await {
                EventReceive::Event(event) if (self.predicate)(&event) => {
                    return EventReceive::Event(event);
                }
                EventReceive::Event(_) => {}
                EventReceive::Lagged(count) => return EventReceive::Lagged(count),
                EventReceive::Closed => return EventReceive::Closed,
            }
        }
    }

    /// Receive the next matching event while explicitly accepting event loss.
    pub async fn recv_lossy(&mut self) -> Option<E> {
        loop {
            match self.recv_outcome().await {
                EventReceive::Event(event) => return Some(event),
                EventReceive::Lagged(count) => {
                    tracing::warn!(count, "lossy filtered subscription dropped events");
                }
                EventReceive::Closed => return None,
            }
        }
    }

    /// Receive one filtered outcome without hiding loss.
    pub async fn recv(&mut self) -> EventReceive<E> {
        self.recv_outcome().await
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
