//! Typed ARI events deserialized from WebSocket JSON.

use serde::{Deserialize, Serialize};

/// all known ARI event types
///
/// uses serde's internally tagged representation keyed on the `type` field.
/// unknown event types deserialize to the `Unknown` variant instead of failing.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum AriEvent {
    StasisStart {
        channel: Channel,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        replace_channel: Option<Channel>,
    },
    StasisEnd {
        channel: Channel,
    },
    ChannelCreated {
        channel: Channel,
    },
    ChannelDestroyed {
        channel: Channel,
        cause: i32,
        cause_txt: String,
    },
    ChannelStateChange {
        channel: Channel,
    },
    ChannelDtmfReceived {
        channel: Channel,
        digit: String,
        duration_ms: u32,
    },
    ChannelHangupRequest {
        channel: Channel,
    },
    ChannelVarset {
        channel: Option<Channel>,
        variable: String,
        value: String,
    },
    BridgeCreated {
        bridge: Bridge,
    },
    BridgeDestroyed {
        bridge: Bridge,
    },
    ChannelEnteredBridge {
        bridge: Bridge,
        channel: Channel,
    },
    ChannelLeftBridge {
        bridge: Bridge,
        channel: Channel,
    },
    PlaybackStarted {
        playback: Playback,
    },
    PlaybackFinished {
        playback: Playback,
    },
    RecordingStarted {
        recording: LiveRecording,
    },
    RecordingFinished {
        recording: LiveRecording,
    },
    /// catch-all for event types not yet modeled
    #[serde(other)]
    Unknown,
}

impl asterisk_rs_core::event::Event for AriEvent {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub state: String,
    #[serde(default)]
    pub caller: CallerId,
    #[serde(default)]
    pub connected: CallerId,
    #[serde(default)]
    pub dialplan: DialplanCep,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CallerId {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub number: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DialplanCep {
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub exten: String,
    #[serde(default)]
    pub priority: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bridge {
    pub id: String,
    pub technology: String,
    pub bridge_type: String,
    #[serde(default)]
    pub channels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playback {
    pub id: String,
    pub media_uri: String,
    pub state: String,
    #[serde(default)]
    pub target_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveRecording {
    pub name: String,
    pub format: String,
    pub state: String,
    #[serde(default)]
    pub target_uri: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_stasis_start() {
        let json = r#"{
            "type": "StasisStart",
            "channel": {
                "id": "1234.5",
                "name": "PJSIP/alice-00000001",
                "state": "Ring",
                "caller": { "name": "Alice", "number": "1001" },
                "connected": { "name": "", "number": "" },
                "dialplan": { "context": "default", "exten": "100", "priority": 1 }
            },
            "args": ["arg1", "arg2"]
        }"#;

        let event: AriEvent = serde_json::from_str(json)
            .expect("stasis start should deserialize");

        match event {
            AriEvent::StasisStart { channel, args, replace_channel } => {
                assert_eq!(channel.id, "1234.5");
                assert_eq!(channel.name, "PJSIP/alice-00000001");
                assert_eq!(channel.state, "Ring");
                assert_eq!(channel.caller.name, "Alice");
                assert_eq!(channel.caller.number, "1001");
                assert_eq!(args, vec!["arg1", "arg2"]);
                assert!(replace_channel.is_none());
            }
            other => panic!("expected StasisStart, got {other:?}"),
        }
    }

    #[test]
    fn deserialize_channel_dtmf_received() {
        let json = r#"{
            "type": "ChannelDtmfReceived",
            "channel": {
                "id": "chan-1",
                "name": "PJSIP/bob-00000002",
                "state": "Up"
            },
            "digit": "5",
            "duration_ms": 120
        }"#;

        let event: AriEvent = serde_json::from_str(json)
            .expect("dtmf received should deserialize");

        match event {
            AriEvent::ChannelDtmfReceived { channel, digit, duration_ms } => {
                assert_eq!(channel.id, "chan-1");
                assert_eq!(digit, "5");
                assert_eq!(duration_ms, 120);
            }
            other => panic!("expected ChannelDtmfReceived, got {other:?}"),
        }
    }

    #[test]
    fn deserialize_unknown_event() {
        let json = r#"{
            "type": "SomeNewEventType",
            "data": "whatever"
        }"#;

        let event: AriEvent = serde_json::from_str(json)
            .expect("unknown event types should not fail deserialization");

        assert!(matches!(event, AriEvent::Unknown));
    }

    #[test]
    fn deserialize_stasis_start_minimal() {
        // no optional fields provided
        let json = r#"{
            "type": "StasisStart",
            "channel": {
                "id": "abc",
                "name": "SIP/trunk-00000001",
                "state": "Ring"
            }
        }"#;

        let event: AriEvent = serde_json::from_str(json)
            .expect("minimal stasis start should deserialize");

        match event {
            AriEvent::StasisStart { channel, args, replace_channel } => {
                assert_eq!(channel.id, "abc");
                assert!(args.is_empty());
                assert!(replace_channel.is_none());
            }
            other => panic!("expected StasisStart, got {other:?}"),
        }
    }
}
