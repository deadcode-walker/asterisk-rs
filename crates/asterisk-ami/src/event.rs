//! typed AMI event types

use crate::codec::RawAmiMessage;
use std::collections::HashMap;

/// all known AMI event types
#[derive(Debug, Clone)]
pub enum AmiEvent {
    /// new channel created
    NewChannel {
        channel: String,
        channel_state: String,
        channel_state_desc: String,
        caller_id_num: String,
        caller_id_name: String,
        unique_id: String,
        linked_id: String,
    },

    /// channel hung up
    Hangup {
        channel: String,
        unique_id: String,
        cause: u32,
        cause_txt: String,
    },

    /// channel state changed
    Newstate {
        channel: String,
        channel_state: String,
        channel_state_desc: String,
        unique_id: String,
    },

    /// dial begin
    DialBegin {
        channel: String,
        destination: String,
        dial_string: String,
        unique_id: String,
        dest_unique_id: String,
    },

    /// dial end
    DialEnd {
        channel: String,
        destination: String,
        dial_status: String,
        unique_id: String,
        dest_unique_id: String,
    },

    /// DTMF digit received
    DtmfBegin {
        channel: String,
        digit: String,
        direction: String,
        unique_id: String,
    },

    /// DTMF digit ended
    DtmfEnd {
        channel: String,
        digit: String,
        duration_ms: u32,
        direction: String,
        unique_id: String,
    },

    /// asterisk has finished booting
    FullyBooted {
        status: String,
    },

    /// peer registration/status change
    PeerStatus {
        channel_type: String,
        peer: String,
        peer_status: String,
    },

    /// bridge created
    BridgeCreate {
        bridge_unique_id: String,
        bridge_type: String,
    },

    /// bridge destroyed
    BridgeDestroy {
        bridge_unique_id: String,
    },

    /// channel entered bridge
    BridgeEnter {
        bridge_unique_id: String,
        channel: String,
        unique_id: String,
    },

    /// channel left bridge
    BridgeLeave {
        bridge_unique_id: String,
        channel: String,
        unique_id: String,
    },

    /// unrecognized event — carries all raw headers
    Unknown {
        event_name: String,
        headers: HashMap<String, String>,
    },
}

impl AmiEvent {
    /// parse an AMI event from a raw message
    ///
    /// returns `None` if the message is not an event
    pub fn from_raw(raw: &RawAmiMessage) -> Option<Self> {
        let event_name = raw.get("Event")?;

        let event = match event_name {
            "Newchannel" => Self::NewChannel {
                channel: get(raw, "Channel"),
                channel_state: get(raw, "ChannelState"),
                channel_state_desc: get(raw, "ChannelStateDesc"),
                caller_id_num: get(raw, "CallerIDNum"),
                caller_id_name: get(raw, "CallerIDName"),
                unique_id: get(raw, "Uniqueid"),
                linked_id: get(raw, "Linkedid"),
            },
            "Hangup" => Self::Hangup {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                cause: raw
                    .get("Cause")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                cause_txt: get(raw, "Cause-txt"),
            },
            "Newstate" => Self::Newstate {
                channel: get(raw, "Channel"),
                channel_state: get(raw, "ChannelState"),
                channel_state_desc: get(raw, "ChannelStateDesc"),
                unique_id: get(raw, "Uniqueid"),
            },
            "DialBegin" => Self::DialBegin {
                channel: get(raw, "Channel"),
                destination: get(raw, "DestChannel"),
                dial_string: get(raw, "DialString"),
                unique_id: get(raw, "Uniqueid"),
                dest_unique_id: get(raw, "DestUniqueid"),
            },
            "DialEnd" => Self::DialEnd {
                channel: get(raw, "Channel"),
                destination: get(raw, "DestChannel"),
                dial_status: get(raw, "DialStatus"),
                unique_id: get(raw, "Uniqueid"),
                dest_unique_id: get(raw, "DestUniqueid"),
            },
            "DTMFBegin" => Self::DtmfBegin {
                channel: get(raw, "Channel"),
                digit: get(raw, "Digit"),
                direction: get(raw, "Direction"),
                unique_id: get(raw, "Uniqueid"),
            },
            "DTMFEnd" => Self::DtmfEnd {
                channel: get(raw, "Channel"),
                digit: get(raw, "Digit"),
                duration_ms: raw
                    .get("DurationMs")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                direction: get(raw, "Direction"),
                unique_id: get(raw, "Uniqueid"),
            },
            "FullyBooted" => Self::FullyBooted {
                status: get(raw, "Status"),
            },
            "PeerStatus" => Self::PeerStatus {
                channel_type: get(raw, "ChannelType"),
                peer: get(raw, "Peer"),
                peer_status: get(raw, "PeerStatus"),
            },
            "BridgeCreate" => Self::BridgeCreate {
                bridge_unique_id: get(raw, "BridgeUniqueid"),
                bridge_type: get(raw, "BridgeType"),
            },
            "BridgeDestroy" => Self::BridgeDestroy {
                bridge_unique_id: get(raw, "BridgeUniqueid"),
            },
            "BridgeEnter" => Self::BridgeEnter {
                bridge_unique_id: get(raw, "BridgeUniqueid"),
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
            },
            "BridgeLeave" => Self::BridgeLeave {
                bridge_unique_id: get(raw, "BridgeUniqueid"),
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
            },
            _ => Self::Unknown {
                event_name: event_name.to_string(),
                headers: raw.to_map(),
            },
        };

        Some(event)
    }

    /// the event type name
    pub fn event_name(&self) -> &str {
        match self {
            Self::NewChannel { .. } => "Newchannel",
            Self::Hangup { .. } => "Hangup",
            Self::Newstate { .. } => "Newstate",
            Self::DialBegin { .. } => "DialBegin",
            Self::DialEnd { .. } => "DialEnd",
            Self::DtmfBegin { .. } => "DTMFBegin",
            Self::DtmfEnd { .. } => "DTMFEnd",
            Self::FullyBooted { .. } => "FullyBooted",
            Self::PeerStatus { .. } => "PeerStatus",
            Self::BridgeCreate { .. } => "BridgeCreate",
            Self::BridgeDestroy { .. } => "BridgeDestroy",
            Self::BridgeEnter { .. } => "BridgeEnter",
            Self::BridgeLeave { .. } => "BridgeLeave",
            Self::Unknown { event_name, .. } => event_name,
        }
    }

    /// get the channel name, if this event pertains to a channel
    pub fn channel(&self) -> Option<&str> {
        match self {
            Self::NewChannel { channel, .. }
            | Self::Hangup { channel, .. }
            | Self::Newstate { channel, .. }
            | Self::DialBegin { channel, .. }
            | Self::DialEnd { channel, .. }
            | Self::DtmfBegin { channel, .. }
            | Self::DtmfEnd { channel, .. }
            | Self::BridgeEnter { channel, .. }
            | Self::BridgeLeave { channel, .. } => Some(channel),
            _ => None,
        }
    }

    /// get the unique id, if this event carries one
    pub fn unique_id(&self) -> Option<&str> {
        match self {
            Self::NewChannel { unique_id, .. }
            | Self::Hangup { unique_id, .. }
            | Self::Newstate { unique_id, .. }
            | Self::DialBegin { unique_id, .. }
            | Self::DialEnd { unique_id, .. }
            | Self::DtmfBegin { unique_id, .. }
            | Self::DtmfEnd { unique_id, .. }
            | Self::BridgeEnter { unique_id, .. }
            | Self::BridgeLeave { unique_id, .. } => Some(unique_id),
            _ => None,
        }
    }
}

// AmiEvent works with the core EventBus
impl asterisk_rs_core::event::Event for AmiEvent {}

/// extract a header value or return empty string
fn get(raw: &RawAmiMessage, key: &str) -> String {
    raw.get(key).unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::RawAmiMessage;

    #[test]
    fn parse_hangup_event() {
        let raw = RawAmiMessage {
            headers: vec![
                ("Event".into(), "Hangup".into()),
                ("Channel".into(), "SIP/100-0001".into()),
                ("Uniqueid".into(), "1234.5".into()),
                ("Cause".into(), "16".into()),
                ("Cause-txt".into(), "Normal Clearing".into()),
            ],
        };
        let event = AmiEvent::from_raw(&raw).expect("should parse hangup event");
        assert_eq!(event.event_name(), "Hangup");
        assert_eq!(event.channel(), Some("SIP/100-0001"));
        if let AmiEvent::Hangup {
            cause, cause_txt, ..
        } = &event
        {
            assert_eq!(*cause, 16);
            assert_eq!(cause_txt, "Normal Clearing");
        } else {
            panic!("expected Hangup variant");
        }
    }

    #[test]
    fn parse_unknown_event() {
        let raw = RawAmiMessage {
            headers: vec![
                ("Event".into(), "CustomEvent".into()),
                ("Data".into(), "something".into()),
            ],
        };
        let event = AmiEvent::from_raw(&raw).expect("should parse unknown event");
        assert_eq!(event.event_name(), "CustomEvent");
        assert!(matches!(event, AmiEvent::Unknown { .. }));
    }

    #[test]
    fn non_event_returns_none() {
        let raw = RawAmiMessage {
            headers: vec![("Response".into(), "Success".into())],
        };
        assert!(AmiEvent::from_raw(&raw).is_none());
    }
}
