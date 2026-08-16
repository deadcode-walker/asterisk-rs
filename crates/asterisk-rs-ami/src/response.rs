//! AMI response types and ActionID correlation

use crate::codec::RawAmiMessage;
use crate::error::{AmiError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use tokio::time::Instant;

const REQUEST_QUEUED: u8 = 0;
const REQUEST_WRITING: u8 = 1;
const REQUEST_SENT: u8 = 2;
const REQUEST_COMPLETED: u8 = 3;
const REQUEST_CANCELLED: u8 = 4;

/// coordinates cancellation between a caller and the connection actor
#[derive(Debug, Default)]
pub(crate) struct RequestLifecycle(AtomicU8);

impl RequestLifecycle {
    /// claim the request for its first wire write
    pub(crate) fn begin_write(&self) -> bool {
        self.0
            .compare_exchange(
                REQUEST_QUEUED,
                REQUEST_WRITING,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    /// cancel a request that has not started writing
    pub(crate) fn cancel_queued(&self) -> bool {
        self.0
            .compare_exchange(
                REQUEST_QUEUED,
                REQUEST_CANCELLED,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    pub(crate) fn mark_sent(&self) {
        self.0.store(REQUEST_SENT, Ordering::Release);
    }

    pub(crate) fn mark_completed(&self) {
        self.0.store(REQUEST_COMPLETED, Ordering::Release);
    }

    pub(crate) fn may_have_executed(&self) -> bool {
        matches!(
            self.0.load(Ordering::Acquire),
            REQUEST_WRITING | REQUEST_SENT | REQUEST_COMPLETED
        )
    }
}

struct ManagedRequest<T> {
    deadline: Instant,
    lifecycle: Arc<RequestLifecycle>,
    tx: tokio::sync::oneshot::Sender<Result<T>>,
}

enum ResponseSender {
    Public(tokio::sync::oneshot::Sender<AmiResponse>),
    Managed(ManagedRequest<AmiResponse>),
}

/// parsed AMI response
#[derive(Debug, Clone, PartialEq)]
pub struct AmiResponse {
    /// the ActionID this response corresponds to
    pub action_id: String,
    /// whether the action succeeded
    pub success: bool,
    /// the Response header value ("Success", "Error", "Follows")
    pub response_type: String,
    /// the Message header, if present
    pub message: Option<String>,
    /// all headers as a map
    pub headers: HashMap<String, String>,
    /// command output lines (populated for Response: Follows)
    pub output: Vec<String>,
    /// channel variables extracted from ChanVariable(name) headers
    pub channel_variables: HashMap<String, String>,
}

impl AmiResponse {
    /// parse a response from a raw AMI message
    ///
    /// returns `None` for non-response messages (e.g., events)
    pub fn from_raw(raw: &RawAmiMessage) -> Option<Self> {
        // messages with both Event: and Response: headers are events
        // (e.g. OriginateResponse carries Response: Success/Failure
        // but is an event, not an action response)
        if raw.get("Event").is_some() {
            return None;
        }
        let response_type = raw.get("Response")?.to_string();
        // action ID may be absent for unsolicited responses
        let action_id = raw.get("ActionID").unwrap_or("").to_string();
        let success = response_type.eq_ignore_ascii_case("success")
            || response_type.eq_ignore_ascii_case("follows");
        let message = raw.get("Message").map(String::from);
        let headers = raw.to_map();

        Some(Self {
            action_id,
            success,
            response_type,
            message,
            headers,
            output: raw.output.clone(),
            channel_variables: raw.channel_variables.clone(),
        })
    }

    /// get a header value from the response
    pub fn get(&self, key: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }

    /// get a channel variable by name
    pub fn get_variable(&self, name: &str) -> Option<&str> {
        self.channel_variables.get(name).map(|s| s.as_str())
    }

    fn retained_size(&self) -> usize {
        self.headers
            .iter()
            .map(|(key, value)| key.len() + value.len())
            .sum::<usize>()
            + self.output.iter().map(String::len).sum::<usize>()
            + self
                .channel_variables
                .iter()
                .map(|(name, value)| name.len() + value.len())
                .sum::<usize>()
    }
}

/// response from an event-generating action (e.g., Status, QueueStatus)
///
/// contains the initial response plus all events received until the
/// completion marker event
#[derive(Debug, Clone)]
pub struct EventListResponse {
    /// the initial response to the action
    pub response: AmiResponse,
    /// events received as part of this action's result
    pub events: Vec<crate::event::AmiEvent>,
}

/// maximum events allowed in a single event list before it is dropped
pub const MAX_EVENT_LIST_EVENTS: usize = 10_000;
/// maximum retained payload across one event-generating action (4 MiB)
pub const MAX_EVENT_LIST_BYTES: usize = 4 * 1024 * 1024;
/// maximum event-generating actions awaiting completion on one connection
pub const MAX_IN_FLIGHT_EVENT_LISTS: usize = 16;
/// maximum retained payload across all event-generating actions on one connection (8 MiB)
pub const MAX_CONNECTION_EVENT_LIST_BYTES: usize = 8 * 1024 * 1024;
/// maximum actions awaiting responses on one AMI connection
pub const MAX_IN_FLIGHT_ACTIONS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EventListTerminal {
    Continue,
    Complete,
    Cancelled,
}

/// tracks a pending event-generating action
struct PendingEventList {
    response: Option<AmiResponse>,
    events: Vec<crate::event::AmiEvent>,
    retained_bytes: usize,
    tx: EventListSender,
}

enum EventListSender {
    Public(tokio::sync::oneshot::Sender<EventListResponse>),
    Managed(ManagedRequest<EventListResponse>),
}

/// pending action tracker — correlates ActionIDs with response channels
pub struct PendingActions {
    pending: HashMap<String, ResponseSender>,
    pending_event_lists: HashMap<String, PendingEventList>,
    retained_event_list_bytes: usize,
}

impl PendingActions {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            pending_event_lists: HashMap::new(),
            retained_event_list_bytes: 0,
        }
    }

    /// register a pending action, returns a receiver for the response
    pub fn register(&mut self, action_id: String) -> tokio::sync::oneshot::Receiver<AmiResponse> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending.insert(action_id, ResponseSender::Public(tx));
        rx
    }

    /// deliver a response to the waiting caller
    ///
    /// returns true if the response was delivered, false if no one was waiting
    pub fn deliver(&mut self, response: AmiResponse) -> bool {
        match self.pending.remove(&response.action_id) {
            Some(ResponseSender::Public(tx)) => {
                // send can fail if the receiver was dropped, which is fine
                tx.send(response).is_ok()
            }
            Some(ResponseSender::Managed(request)) => {
                let delivered = request.tx.send(Ok(response)).is_ok();
                request.lifecycle.mark_completed();
                delivered
            }
            None => false,
        }
    }

    /// number of actions waiting for responses
    pub fn pending_count(&self) -> usize {
        self.pending.len() + self.pending_event_lists.len()
    }

    pub(crate) fn at_in_flight_limit(&self) -> bool {
        self.pending_count() >= MAX_IN_FLIGHT_ACTIONS
    }

    pub(crate) fn at_event_list_limit(&self) -> bool {
        self.pending_event_lists.len() >= MAX_IN_FLIGHT_EVENT_LISTS
    }

    /// cancel all pending actions (e.g., on disconnect)
    ///
    /// drops all senders, causing receivers to get `RecvError::Closed`
    pub fn cancel_all(&mut self) {
        self.pending.clear();
        self.pending_event_lists.clear();
        self.retained_event_list_bytes = 0;
    }

    /// register with a pre-existing sender (used by connection manager)
    pub fn register_with_sender(
        &mut self,
        action_id: String,
        tx: tokio::sync::oneshot::Sender<AmiResponse>,
    ) {
        self.pending.insert(action_id, ResponseSender::Public(tx));
    }

    /// register a pending event-generating action
    pub fn register_event_list(
        &mut self,
        action_id: String,
        tx: tokio::sync::oneshot::Sender<EventListResponse>,
    ) {
        self.remove_event_list(&action_id);
        self.pending_event_lists.insert(
            action_id,
            PendingEventList {
                response: None,
                events: Vec::new(),
                retained_bytes: 0,
                tx: EventListSender::Public(tx),
            },
        );
    }

    /// check whether an action_id has a pending event list
    pub fn contains_event_list(&self, action_id: &str) -> bool {
        self.pending_event_lists.contains_key(action_id)
    }

    /// deliver the initial response for an event-generating action
    ///
    /// returns true if this action_id has a pending event list
    pub fn deliver_event_list_response(&mut self, response: AmiResponse) -> bool {
        let retained_bytes = response.retained_size();
        self.deliver_event_list_response_with_size(response, retained_bytes)
    }

    pub(crate) fn deliver_event_list_response_with_size(
        &mut self,
        response: AmiResponse,
        retained_bytes: usize,
    ) -> bool {
        let action_id = response.action_id.clone();
        let Some(pending) = self.pending_event_lists.get_mut(&action_id) else {
            return false;
        };

        if !response.success {
            let Some(pending) = self.remove_event_list(&action_id) else {
                return false;
            };
            finish_event_list(
                pending,
                EventListResponse {
                    response,
                    events: Vec::new(),
                },
            );
            return true;
        }

        if pending.response.is_some() {
            return true;
        }
        if let Err(error) = self.reserve_event_list_bytes(&action_id, retained_bytes) {
            let Some(pending) = self.remove_event_list(&action_id) else {
                return false;
            };
            fail_event_list(pending, error);
            return true;
        }
        self.pending_event_lists
            .get_mut(&action_id)
            .expect("event list exists after reserving its response")
            .response = Some(response);
        true
    }

    /// deliver an event for an event-generating action
    ///
    /// completion is detected via the `EventList: Complete` header that
    /// Asterisk sends on `*Complete` events, rather than matching on the
    /// event name suffix (which could false-positive on user events like
    /// `ProcessComplete`).
    ///
    /// returns true if this event was consumed by a pending event list
    pub fn deliver_event_list_event(
        &mut self,
        action_id: &str,
        event: crate::event::AmiEvent,
    ) -> bool {
        let terminal = event_list_terminal(&event);
        self.deliver_event_list_event_with_metadata(action_id, event, 0, terminal)
    }

    pub(crate) fn deliver_event_list_event_with_metadata(
        &mut self,
        action_id: &str,
        event: crate::event::AmiEvent,
        retained_bytes: usize,
        terminal: EventListTerminal,
    ) -> bool {
        let Some(pending) = self.pending_event_lists.get_mut(action_id) else {
            return false;
        };

        if terminal == EventListTerminal::Cancelled {
            let Some(pending) = self.remove_event_list(action_id) else {
                return false;
            };
            fail_event_list(
                pending,
                AmiError::EventListCancelled {
                    action_id: action_id.to_owned(),
                },
            );
            return true;
        }

        if pending.events.len() >= MAX_EVENT_LIST_EVENTS {
            let Some(pending) = self.remove_event_list(action_id) else {
                return false;
            };
            fail_event_list(
                pending,
                AmiError::EventListEventLimitExceeded {
                    action_id: action_id.to_owned(),
                    limit: MAX_EVENT_LIST_EVENTS,
                },
            );
            return true;
        }

        if let Err(error) = self.reserve_event_list_bytes(action_id, retained_bytes) {
            let Some(pending) = self.remove_event_list(action_id) else {
                return false;
            };
            fail_event_list(pending, error);
            return true;
        }

        self.pending_event_lists
            .get_mut(action_id)
            .expect("event list exists after reserving its event")
            .events
            .push(event);
        if terminal == EventListTerminal::Complete {
            let Some(pending) = self.remove_event_list(action_id) else {
                return false;
            };
            let mut pending = pending;
            let response = pending.response.take().unwrap_or_else(|| {
                tracing::warn!(action_id, "event list Complete arrived before Response");
                AmiResponse {
                    action_id: action_id.to_owned(),
                    success: false,
                    response_type: String::new(),
                    message: Some("event list completed before response received".into()),
                    headers: HashMap::new(),
                    output: Vec::new(),
                    channel_variables: HashMap::new(),
                }
            });
            let events = std::mem::take(&mut pending.events);
            finish_event_list(pending, EventListResponse { response, events });
            true
        } else {
            true
        }
    }

    pub(crate) fn register_managed(
        &mut self,
        action_id: String,
        deadline: Instant,
        lifecycle: Arc<RequestLifecycle>,
        tx: tokio::sync::oneshot::Sender<Result<AmiResponse>>,
    ) {
        self.pending.insert(
            action_id,
            ResponseSender::Managed(ManagedRequest {
                deadline,
                lifecycle,
                tx,
            }),
        );
    }

    pub(crate) fn register_managed_event_list(
        &mut self,
        action_id: String,
        deadline: Instant,
        lifecycle: Arc<RequestLifecycle>,
        tx: tokio::sync::oneshot::Sender<Result<EventListResponse>>,
    ) {
        self.remove_event_list(&action_id);
        self.pending_event_lists.insert(
            action_id,
            PendingEventList {
                response: None,
                events: Vec::new(),
                retained_bytes: 0,
                tx: EventListSender::Managed(ManagedRequest {
                    deadline,
                    lifecycle,
                    tx,
                }),
            },
        );
    }

    /// discard requests whose callers have gone away
    pub fn purge_closed(&mut self) {
        self.pending.retain(|_, sender| match sender {
            ResponseSender::Public(tx) => !tx.is_closed(),
            ResponseSender::Managed(request) => !request.tx.is_closed(),
        });
        let closed_event_lists: Vec<_> = self
            .pending_event_lists
            .iter()
            .filter_map(|(action_id, pending)| match &pending.tx {
                EventListSender::Public(tx) if tx.is_closed() => Some(action_id.clone()),
                EventListSender::Managed(request) if request.tx.is_closed() => {
                    Some(action_id.clone())
                }
                _ => None,
            })
            .collect();
        for action_id in closed_event_lists {
            self.remove_event_list(&action_id);
        }
    }

    pub(crate) fn next_deadline(&self) -> Option<Instant> {
        self.pending
            .values()
            .filter_map(|sender| match sender {
                ResponseSender::Public(_) => None,
                ResponseSender::Managed(request) => Some(request.deadline),
            })
            .chain(
                self.pending_event_lists
                    .values()
                    .filter_map(|pending| match &pending.tx {
                        EventListSender::Public(_) => None,
                        EventListSender::Managed(request) => Some(request.deadline),
                    }),
            )
            .min()
    }

    /// expire sent requests whose response deadline elapsed
    pub(crate) fn expire(&mut self, now: Instant) {
        self.purge_closed();

        let expired_actions: Vec<_> = self
            .pending
            .iter()
            .filter_map(|(action_id, sender)| match sender {
                ResponseSender::Managed(request) if request.deadline <= now => {
                    Some(action_id.clone())
                }
                _ => None,
            })
            .collect();
        for action_id in expired_actions {
            let Some(ResponseSender::Managed(request)) = self.pending.remove(&action_id) else {
                continue;
            };
            let _ = request.tx.send(Err(AmiError::OutcomeUnknown {
                action_id: action_id.clone(),
            }));
            request.lifecycle.mark_completed();
        }

        let expired_event_lists: Vec<_> = self
            .pending_event_lists
            .iter()
            .filter_map(|(action_id, pending)| match &pending.tx {
                EventListSender::Managed(request) if request.deadline <= now => {
                    Some(action_id.clone())
                }
                _ => None,
            })
            .collect();
        for action_id in expired_event_lists {
            let Some(pending) = self.remove_event_list(&action_id) else {
                continue;
            };
            let EventListSender::Managed(request) = pending.tx else {
                continue;
            };
            let _ = request.tx.send(Err(AmiError::OutcomeUnknown {
                action_id: action_id.clone(),
            }));
            request.lifecycle.mark_completed();
        }
    }

    /// fail all wire-started requests because the connection was lost
    pub(crate) fn fail_all_unknown(&mut self) {
        for (action_id, sender) in self.pending.drain() {
            if let ResponseSender::Managed(request) = sender {
                let _ = request.tx.send(Err(AmiError::OutcomeUnknown { action_id }));
                request.lifecycle.mark_completed();
            }
        }
        for (action_id, pending) in self.pending_event_lists.drain() {
            if let EventListSender::Managed(request) = pending.tx {
                let _ = request.tx.send(Err(AmiError::OutcomeUnknown { action_id }));
                request.lifecycle.mark_completed();
            }
        }
        self.retained_event_list_bytes = 0;
    }

    fn reserve_event_list_bytes(&mut self, action_id: &str, additional: usize) -> Result<()> {
        let retained_bytes = self
            .pending_event_lists
            .get(action_id)
            .expect("cannot reserve bytes for an unknown event list")
            .retained_bytes;
        let collector_total = retained_bytes.saturating_add(additional);
        if collector_total > MAX_EVENT_LIST_BYTES {
            return Err(AmiError::EventListByteLimitExceeded {
                action_id: action_id.to_owned(),
                limit: MAX_EVENT_LIST_BYTES,
            });
        }

        let connection_total = self.retained_event_list_bytes.saturating_add(additional);
        if connection_total > MAX_CONNECTION_EVENT_LIST_BYTES {
            return Err(AmiError::EventListConnectionByteLimitExceeded {
                action_id: action_id.to_owned(),
                limit: MAX_CONNECTION_EVENT_LIST_BYTES,
            });
        }

        self.retained_event_list_bytes = connection_total;
        self.pending_event_lists
            .get_mut(action_id)
            .expect("event list exists while reserving bytes")
            .retained_bytes = collector_total;
        Ok(())
    }

    fn remove_event_list(&mut self, action_id: &str) -> Option<PendingEventList> {
        let pending = self.pending_event_lists.remove(action_id)?;
        self.retained_event_list_bytes = self
            .retained_event_list_bytes
            .checked_sub(pending.retained_bytes)
            .expect("event-list retained-byte accounting must not underflow");
        Some(pending)
    }
}

impl Default for PendingActions {
    fn default() -> Self {
        Self::new()
    }
}

fn finish_event_list(pending: PendingEventList, result: EventListResponse) {
    match pending.tx {
        EventListSender::Public(tx) => {
            let _ = tx.send(result);
        }
        EventListSender::Managed(request) => {
            let _ = request.tx.send(Ok(result));
            request.lifecycle.mark_completed();
        }
    }
}

fn fail_event_list(pending: PendingEventList, error: AmiError) {
    if let EventListSender::Managed(request) = pending.tx {
        let _ = request.tx.send(Err(error));
        request.lifecycle.mark_completed();
    }
}

fn event_list_terminal(event: &crate::event::AmiEvent) -> EventListTerminal {
    if event.is_event_list_complete() {
        return EventListTerminal::Complete;
    }
    if let crate::event::AmiEvent::Unknown { headers, .. } = event {
        if headers.iter().any(|(key, value)| {
            key.eq_ignore_ascii_case("EventList") && value.eq_ignore_ascii_case("Cancelled")
        }) {
            return EventListTerminal::Cancelled;
        }
    }
    EventListTerminal::Continue
}
