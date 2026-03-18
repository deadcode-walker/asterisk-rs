//! AMI response types and ActionID correlation

use crate::codec::RawAmiMessage;
use std::collections::HashMap;

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
        self.headers.get(key).map(|s| s.as_str())
    }

    /// get a channel variable by name
    pub fn get_variable(&self, name: &str) -> Option<&str> {
        self.channel_variables.get(name).map(|s| s.as_str())
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

/// tracks a pending event-generating action
struct PendingEventList {
    response: Option<AmiResponse>,
    events: Vec<crate::event::AmiEvent>,
    tx: tokio::sync::oneshot::Sender<EventListResponse>,
}

/// pending action tracker — correlates ActionIDs with response channels
pub struct PendingActions {
    pending: HashMap<String, tokio::sync::oneshot::Sender<AmiResponse>>,
    pending_event_lists: HashMap<String, PendingEventList>,
}

impl PendingActions {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            pending_event_lists: HashMap::new(),
        }
    }

    /// register a pending action, returns a receiver for the response
    pub fn register(&mut self, action_id: String) -> tokio::sync::oneshot::Receiver<AmiResponse> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending.insert(action_id, tx);
        rx
    }

    /// deliver a response to the waiting caller
    ///
    /// returns true if the response was delivered, false if no one was waiting
    pub fn deliver(&mut self, response: AmiResponse) -> bool {
        if let Some(tx) = self.pending.remove(&response.action_id) {
            // send can fail if the receiver was dropped, which is fine
            tx.send(response).is_ok()
        } else {
            false
        }
    }

    /// number of actions waiting for responses
    pub fn pending_count(&self) -> usize {
        self.pending.len() + self.pending_event_lists.len()
    }

    /// cancel all pending actions (e.g., on disconnect)
    ///
    /// drops all senders, causing receivers to get `RecvError::Closed`
    pub fn cancel_all(&mut self) {
        self.pending.clear();
        self.pending_event_lists.clear();
    }

    /// register with a pre-existing sender (used by connection manager)
    pub fn register_with_sender(
        &mut self,
        action_id: String,
        tx: tokio::sync::oneshot::Sender<AmiResponse>,
    ) {
        self.pending.insert(action_id, tx);
    }

    /// register a pending event-generating action
    pub fn register_event_list(
        &mut self,
        action_id: String,
        tx: tokio::sync::oneshot::Sender<EventListResponse>,
    ) {
        self.pending_event_lists.insert(
            action_id,
            PendingEventList {
                response: None,
                events: Vec::new(),
                tx,
            },
        );
    }

    /// deliver the initial response for an event-generating action
    ///
    /// returns true if this action_id has a pending event list
    pub fn deliver_event_list_response(&mut self, response: AmiResponse) -> bool {
        if let Some(pending) = self.pending_event_lists.get_mut(&response.action_id) {
            pending.response = Some(response);
            true
        } else {
            false
        }
    }

    /// deliver an event for an event-generating action
    ///
    /// returns true if this event was consumed by a pending event list
    pub fn deliver_event_list_event(
        &mut self,
        action_id: &str,
        event: crate::event::AmiEvent,
    ) -> bool {
        let is_complete = event.event_name().ends_with("Complete");

        if let Some(mut pending) = if is_complete {
            self.pending_event_lists.remove(action_id)
        } else {
            None
        } {
            pending.events.push(event);
            let response = pending.response.unwrap_or_else(|| AmiResponse {
                action_id: action_id.to_string(),
                success: true,
                response_type: String::new(),
                message: None,
                headers: HashMap::new(),
                output: vec![],
                channel_variables: HashMap::new(),
            });
            let _ = pending.tx.send(EventListResponse {
                response,
                events: pending.events,
            });
            return true;
        }

        if let Some(pending) = self.pending_event_lists.get_mut(action_id) {
            pending.events.push(event);
            return true;
        }

        false
    }
}

impl Default for PendingActions {
    fn default() -> Self {
        Self::new()
    }
}

