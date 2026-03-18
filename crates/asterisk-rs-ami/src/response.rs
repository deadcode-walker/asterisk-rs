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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::RawAmiMessage;

    #[test]
    fn parse_success_response() {
        let raw = RawAmiMessage {
            headers: vec![
                ("Response".into(), "Success".into()),
                ("ActionID".into(), "42".into()),
                ("Message".into(), "Authentication accepted".into()),
            ],
            output: vec![],
            channel_variables: HashMap::new(),
        };
        let resp = AmiResponse::from_raw(&raw).expect("should parse success response");
        assert!(resp.success);
        assert_eq!(resp.action_id, "42");
        assert_eq!(resp.message.as_deref(), Some("Authentication accepted"));
    }

    #[test]
    fn parse_error_response() {
        let raw = RawAmiMessage {
            headers: vec![
                ("Response".into(), "Error".into()),
                ("ActionID".into(), "43".into()),
                ("Message".into(), "Permission denied".into()),
            ],
            output: vec![],
            channel_variables: HashMap::new(),
        };
        let resp = AmiResponse::from_raw(&raw).expect("should parse error response");
        assert!(!resp.success);
        assert_eq!(resp.message.as_deref(), Some("Permission denied"));
    }

    #[test]
    fn returns_none_for_event_message() {
        let raw = RawAmiMessage {
            headers: vec![
                ("Event".into(), "Hangup".into()),
                ("Channel".into(), "SIP/100-00000001".into()),
            ],
            output: vec![],
            channel_variables: HashMap::new(),
        };
        assert!(AmiResponse::from_raw(&raw).is_none());
    }

    #[test]
    fn pending_actions_lifecycle() {
        let mut pending = PendingActions::new();
        let mut rx = pending.register("1".into());
        assert_eq!(pending.pending_count(), 1);

        let response = AmiResponse {
            action_id: "1".into(),
            success: true,
            response_type: "Success".into(),
            message: None,
            headers: HashMap::new(),
            output: vec![],
            channel_variables: HashMap::new(),
        };
        assert!(pending.deliver(response));
        assert_eq!(pending.pending_count(), 0);

        // receiver should have the response
        let received = rx.try_recv().expect("should receive response");
        assert!(received.success);
    }

    #[test]
    fn deliver_unknown_action_id_returns_false() {
        let mut pending = PendingActions::new();
        let response = AmiResponse {
            action_id: "unknown".into(),
            success: true,
            response_type: "Success".into(),
            message: None,
            headers: HashMap::new(),
            output: vec![],
            channel_variables: HashMap::new(),
        };
        assert!(!pending.deliver(response));
    }

    #[test]
    fn cancel_all_clears_pending() {
        let mut pending = PendingActions::new();
        let _rx1 = pending.register("1".into());
        let _rx2 = pending.register("2".into());
        assert_eq!(pending.pending_count(), 2);

        pending.cancel_all();
        assert_eq!(pending.pending_count(), 0);
    }

    #[test]
    fn event_list_lifecycle() {
        let mut pending = PendingActions::new();
        let (tx, mut rx) = tokio::sync::oneshot::channel();
        pending.register_event_list("100".into(), tx);

        // deliver initial response
        let response = AmiResponse {
            action_id: "100".into(),
            success: true,
            response_type: "Success".into(),
            message: None,
            headers: HashMap::new(),
            output: vec![],
            channel_variables: HashMap::new(),
        };
        assert!(pending.deliver_event_list_response(response));

        // deliver intermediate event
        let event = crate::event::AmiEvent::Unknown {
            event_name: "Status".into(),
            headers: HashMap::new(),
        };
        assert!(pending.deliver_event_list_event("100", event));

        // deliver completion event
        let complete = crate::event::AmiEvent::Unknown {
            event_name: "StatusComplete".into(),
            headers: HashMap::new(),
        };
        assert!(pending.deliver_event_list_event("100", complete));

        // should have received the result
        let result = rx.try_recv().expect("should have result");
        assert!(result.response.success);
        assert_eq!(result.events.len(), 2);
    }

    #[test]
    fn event_list_does_not_steal_unrelated_events() {
        let mut pending = PendingActions::new();
        let (tx, _rx) = tokio::sync::oneshot::channel();
        pending.register_event_list("200".into(), tx);

        let event = crate::event::AmiEvent::Unknown {
            event_name: "Hangup".into(),
            headers: HashMap::new(),
        };
        // action_id doesn't match
        assert!(!pending.deliver_event_list_event("999", event));
    }

    #[test]
    fn response_propagates_channel_variables() {
        let mut vars = HashMap::new();
        vars.insert("DIALSTATUS".to_string(), "ANSWER".to_string());
        let raw = RawAmiMessage {
            headers: vec![
                ("Response".into(), "Success".into()),
                ("ActionID".into(), "99".into()),
            ],
            output: vec![],
            channel_variables: vars,
        };
        let resp = AmiResponse::from_raw(&raw).expect("should parse");
        assert_eq!(resp.get_variable("DIALSTATUS"), Some("ANSWER"));
    }

    #[test]
    fn parse_follows_response_is_success() {
        let raw = RawAmiMessage {
            headers: vec![
                ("Response".into(), "Follows".into()),
                ("ActionID".into(), "50".into()),
            ],
            output: vec![],
            channel_variables: HashMap::new(),
        };
        let resp = AmiResponse::from_raw(&raw).expect("should parse follows response");
        assert!(resp.success);
        assert_eq!(resp.response_type, "Follows");
    }

    #[test]
    fn parse_response_case_insensitive() {
        let raw = RawAmiMessage {
            headers: vec![
                ("Response".into(), "success".into()),
                ("ActionID".into(), "51".into()),
            ],
            output: vec![],
            channel_variables: HashMap::new(),
        };
        let resp = AmiResponse::from_raw(&raw).expect("should parse lowercase success");
        assert!(resp.success);
    }

    #[test]
    fn parse_response_without_action_id() {
        let raw = RawAmiMessage {
            headers: vec![("Response".into(), "Success".into())],
            output: vec![],
            channel_variables: HashMap::new(),
        };
        let resp = AmiResponse::from_raw(&raw).expect("should parse without action id");
        assert_eq!(resp.action_id, "");
    }

    #[test]
    fn parse_response_without_message() {
        let raw = RawAmiMessage {
            headers: vec![
                ("Response".into(), "Success".into()),
                ("ActionID".into(), "52".into()),
            ],
            output: vec![],
            channel_variables: HashMap::new(),
        };
        let resp = AmiResponse::from_raw(&raw).expect("should parse without message");
        assert!(resp.message.is_none());
    }

    #[test]
    fn parse_response_with_output_lines() {
        let raw = RawAmiMessage {
            headers: vec![
                ("Response".into(), "Follows".into()),
                ("ActionID".into(), "53".into()),
            ],
            output: vec!["sip show peers".into(), "Name  Host  Status".into()],
            channel_variables: HashMap::new(),
        };
        let resp = AmiResponse::from_raw(&raw).expect("should parse follows with output");
        assert!(resp.success);
        assert_eq!(resp.output.len(), 2);
        assert_eq!(resp.output[0], "sip show peers");
        assert_eq!(resp.output[1], "Name  Host  Status");
    }

    #[test]
    fn response_get_header() {
        let raw = RawAmiMessage {
            headers: vec![
                ("Response".into(), "Success".into()),
                ("ActionID".into(), "54".into()),
                ("Ping".into(), "Pong".into()),
            ],
            output: vec![],
            channel_variables: HashMap::new(),
        };
        let resp = AmiResponse::from_raw(&raw).expect("should parse");
        assert_eq!(resp.get("Ping"), Some("Pong"));
        assert_eq!(resp.get("Response"), Some("Success"));
    }

    #[test]
    fn response_get_missing_header() {
        let raw = RawAmiMessage {
            headers: vec![("Response".into(), "Success".into())],
            output: vec![],
            channel_variables: HashMap::new(),
        };
        let resp = AmiResponse::from_raw(&raw).expect("should parse");
        assert!(resp.get("NonExistent").is_none());
    }

    #[test]
    fn response_get_variable_missing() {
        let raw = RawAmiMessage {
            headers: vec![("Response".into(), "Success".into())],
            output: vec![],
            channel_variables: HashMap::new(),
        };
        let resp = AmiResponse::from_raw(&raw).expect("should parse");
        assert!(resp.get_variable("MISSING").is_none());
    }

    #[test]
    fn register_same_action_id_replaces() {
        let mut pending = PendingActions::new();
        let _rx1 = pending.register("dup".into());
        let mut rx2 = pending.register("dup".into());
        // only one pending since the key was replaced
        assert_eq!(pending.pending_count(), 1);

        let response = AmiResponse {
            action_id: "dup".into(),
            success: true,
            response_type: "Success".into(),
            message: None,
            headers: HashMap::new(),
            output: vec![],
            channel_variables: HashMap::new(),
        };
        assert!(pending.deliver(response));
        // rx2 should receive the response (it replaced rx1)
        let received = rx2.try_recv().expect("rx2 should receive response");
        assert!(received.success);
    }

    #[test]
    fn deliver_to_dropped_receiver_returns_false() {
        let mut pending = PendingActions::new();
        let rx = pending.register("drop_me".into());
        drop(rx);

        let response = AmiResponse {
            action_id: "drop_me".into(),
            success: true,
            response_type: "Success".into(),
            message: None,
            headers: HashMap::new(),
            output: vec![],
            channel_variables: HashMap::new(),
        };
        // send fails because rx was dropped
        assert!(!pending.deliver(response));
        assert_eq!(pending.pending_count(), 0);
    }

    #[test]
    fn pending_count_includes_event_lists() {
        let mut pending = PendingActions::new();
        let _rx1 = pending.register("regular".into());
        let (tx, _rx2) = tokio::sync::oneshot::channel();
        pending.register_event_list("evlist".into(), tx);
        assert_eq!(pending.pending_count(), 2);
    }

    #[test]
    fn event_list_without_response_uses_default() {
        let mut pending = PendingActions::new();
        let (tx, mut rx) = tokio::sync::oneshot::channel();
        pending.register_event_list("300".into(), tx);

        // deliver completion event without ever delivering a response
        let complete = crate::event::AmiEvent::Unknown {
            event_name: "StatusComplete".into(),
            headers: HashMap::new(),
        };
        assert!(pending.deliver_event_list_event("300", complete));

        let result = rx.try_recv().expect("should have result");
        // default response should have the action_id but empty response_type
        assert_eq!(result.response.action_id, "300");
        assert!(result.response.success);
        assert_eq!(result.response.response_type, "");
        assert!(result.response.message.is_none());
        assert_eq!(result.events.len(), 1);
    }

    #[test]
    fn event_list_intermediate_events_accumulate() {
        let mut pending = PendingActions::new();
        let (tx, mut rx) = tokio::sync::oneshot::channel();
        pending.register_event_list("400".into(), tx);

        let response = AmiResponse {
            action_id: "400".into(),
            success: true,
            response_type: "Success".into(),
            message: None,
            headers: HashMap::new(),
            output: vec![],
            channel_variables: HashMap::new(),
        };
        assert!(pending.deliver_event_list_response(response));

        // deliver 5 intermediate events
        for i in 0..5 {
            let event = crate::event::AmiEvent::Unknown {
                event_name: format!("QueueMember{i}"),
                headers: HashMap::new(),
            };
            assert!(pending.deliver_event_list_event("400", event));
        }

        // deliver completion event
        let complete = crate::event::AmiEvent::Unknown {
            event_name: "QueueMemberComplete".into(),
            headers: HashMap::new(),
        };
        assert!(pending.deliver_event_list_event("400", complete));

        let result = rx.try_recv().expect("should have result");
        // 5 intermediate + 1 completion = 6 events
        assert_eq!(result.events.len(), 6);
    }

    #[test]
    fn cancel_all_clears_event_lists_too() {
        let mut pending = PendingActions::new();
        let _rx1 = pending.register("reg".into());
        let (tx, _rx2) = tokio::sync::oneshot::channel();
        pending.register_event_list("evl".into(), tx);
        assert_eq!(pending.pending_count(), 2);

        pending.cancel_all();
        assert_eq!(pending.pending_count(), 0);
    }

    #[test]
    fn register_with_sender() {
        let mut pending = PendingActions::new();
        let (tx, mut rx) = tokio::sync::oneshot::channel();
        pending.register_with_sender("ext".into(), tx);
        assert_eq!(pending.pending_count(), 1);

        let response = AmiResponse {
            action_id: "ext".into(),
            success: true,
            response_type: "Success".into(),
            message: None,
            headers: HashMap::new(),
            output: vec![],
            channel_variables: HashMap::new(),
        };
        assert!(pending.deliver(response));
        let received = rx.try_recv().expect("should receive via register_with_sender");
        assert!(received.success);
    }

    #[test]
    fn deliver_event_list_event_to_unknown_action_returns_false() {
        let mut pending = PendingActions::new();
        let event = crate::event::AmiEvent::Unknown {
            event_name: "StatusComplete".into(),
            headers: HashMap::new(),
        };
        assert!(!pending.deliver_event_list_event("nonexistent", event));
    }
}
