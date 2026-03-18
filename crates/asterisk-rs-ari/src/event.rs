//! Typed ARI events deserialized from WebSocket JSON.

use serde::{Deserialize, Serialize};

/// all known ARI event types
///
/// uses serde's internally tagged representation keyed on the `type` field.
/// unknown event types deserialize to the `Unknown` variant instead of failing.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
#[non_exhaustive]
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
    /// channel caller id changed
    ChannelCallerId {
        channel: Channel,
        caller_presentation: i32,
        caller_presentation_txt: String,
    },
    /// channel connected line changed
    ChannelConnectedLine {
        channel: Channel,
    },
    /// channel dialplan location changed
    ChannelDialplan {
        channel: Channel,
        dialplan_app: String,
        dialplan_app_data: String,
    },
    /// channel placed on hold
    ChannelHold {
        channel: Channel,
        #[serde(default)]
        musicclass: Option<String>,
    },
    /// channel removed from hold
    ChannelUnhold {
        channel: Channel,
    },
    /// channel talking started
    ChannelTalkingStarted {
        channel: Channel,
    },
    /// channel talking finished
    ChannelTalkingFinished {
        channel: Channel,
        duration: i32,
    },
    /// tone detected on channel
    ChannelToneDetected {
        channel: Channel,
    },
    /// channel transfer via REFER
    ChannelTransfer {
        channel: Channel,
        #[serde(default)]
        refer_to: Option<Box<ReferTo>>,
        #[serde(default)]
        referred_by: Option<Box<ReferredBy>>,
        #[serde(default)]
        state: Option<String>,
    },
    /// user-defined event from the dialplan
    ChannelUserevent {
        #[serde(default)]
        channel: Option<Channel>,
        #[serde(default)]
        bridge: Option<Bridge>,
        #[serde(default)]
        endpoint: Option<Endpoint>,
        eventname: String,
        #[serde(default)]
        userevent: serde_json::Value,
    },
    /// dial event with caller and peer channels
    Dial {
        peer: Channel,
        #[serde(default)]
        caller: Option<Channel>,
        #[serde(default)]
        forwarded: Option<Channel>,
        dialstatus: String,
        #[serde(default)]
        dialstring: Option<String>,
        #[serde(default)]
        forward: Option<String>,
    },
    /// bridge attended transfer completed
    BridgeAttendedTransfer {
        transferer_first_leg: Channel,
        transferer_second_leg: Channel,
        result: String,
        destination_type: String,
        is_external: bool,
        #[serde(default)]
        transferee: Option<Box<Channel>>,
        #[serde(default)]
        transfer_target: Option<Box<Channel>>,
        #[serde(default)]
        replace_channel: Option<Box<Channel>>,
        #[serde(default)]
        transferer_first_leg_bridge: Option<Bridge>,
        #[serde(default)]
        transferer_second_leg_bridge: Option<Bridge>,
        #[serde(default)]
        destination_bridge: Option<String>,
        #[serde(default)]
        destination_application: Option<String>,
        #[serde(default)]
        destination_link_first_leg: Option<Box<Channel>>,
        #[serde(default)]
        destination_link_second_leg: Option<Box<Channel>>,
        #[serde(default)]
        destination_threeway_channel: Option<Box<Channel>>,
        #[serde(default)]
        destination_threeway_bridge: Option<Bridge>,
    },
    /// bridge blind transfer completed
    BridgeBlindTransfer {
        channel: Channel,
        exten: String,
        context: String,
        result: String,
        is_external: bool,
        #[serde(default)]
        bridge: Option<Bridge>,
        #[serde(default)]
        transferee: Option<Channel>,
        #[serde(default)]
        replace_channel: Option<Channel>,
    },
    /// two bridges merged
    BridgeMerged {
        bridge: Bridge,
        bridge_from: Bridge,
    },
    /// bridge video source changed
    BridgeVideoSourceChanged {
        bridge: Bridge,
        #[serde(default)]
        old_video_source_id: Option<String>,
    },
    /// contact status changed
    ContactStatusChange {
        contact_info: ContactInfo,
        endpoint: Endpoint,
    },
    /// device state changed
    DeviceStateChanged {
        device_state: DeviceState,
    },
    /// endpoint state changed
    EndpointStateChange {
        endpoint: Endpoint,
    },
    /// peer status changed
    PeerStatusChange {
        endpoint: Endpoint,
        peer: Peer,
    },
    /// playback continuing to next media uri
    PlaybackContinuing {
        playback: Playback,
    },
    /// recording failed
    RecordingFailed {
        recording: LiveRecording,
    },
    /// application move failed
    ApplicationMoveFailed {
        channel: Channel,
        destination: String,
        #[serde(default)]
        args: Vec<String>,
    },
    /// application registered
    ApplicationRegistered {},
    /// application replaced by another websocket connection
    ApplicationReplaced {},
    /// application unregistered
    ApplicationUnregistered {},
    /// text message received
    TextMessageReceived {
        message: TextMessage,
        #[serde(default)]
        endpoint: Option<Endpoint>,
    },
    /// REST API response over websocket
    RESTResponse {
        status_code: i32,
        reason_phrase: String,
        uri: String,
        request_id: String,
        transaction_id: String,
        #[serde(default)]
        content_type: Option<String>,
        #[serde(default)]
        message_body: Option<String>,
    },
    /// catch-all for event types not yet modeled
    #[serde(other)]
    Unknown,
}

/// a complete ARI event with common metadata and typed payload
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AriMessage {
    /// the stasis application that received this event
    #[serde(default)]
    pub application: String,
    /// ISO 8601 timestamp when the event was created
    #[serde(default)]
    pub timestamp: String,
    /// unique id of the asterisk instance that generated this event
    #[serde(default)]
    pub asterisk_id: Option<String>,
    /// the typed event payload
    #[serde(flatten)]
    pub event: AriEvent,
}

impl asterisk_rs_core::event::Event for AriMessage {}

/// contact info for PJSIP registration status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    pub uri: String,
    pub contact_status: String,
    pub aor: String,
    #[serde(default)]
    pub roundtrip_usec: Option<String>,
}

/// peer status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub peer_status: String,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub port: Option<String>,
    #[serde(default)]
    pub cause: Option<String>,
    #[serde(default)]
    pub time: Option<String>,
}

/// endpoint state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub technology: String,
    pub resource: String,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub channel_ids: Vec<String>,
}

/// device state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceState {
    pub name: String,
    pub state: String,
}

/// text message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextMessage {
    pub from: String,
    pub to: String,
    pub body: String,
}

/// refer-to information for channel transfers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferTo {
    #[serde(default)]
    pub destination_channel: Option<Channel>,
    #[serde(default)]
    pub connected_channel: Option<Channel>,
    #[serde(default)]
    pub bridge: Option<Bridge>,
}

/// referred-by information for channel transfers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferredBy {
    pub source_channel: Channel,
    #[serde(default)]
    pub connected_channel: Option<Channel>,
    #[serde(default)]
    pub bridge: Option<Bridge>,
}

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

        let event: AriEvent = serde_json::from_str(json).expect("stasis start should deserialize");

        match event {
            AriEvent::StasisStart {
                channel,
                args,
                replace_channel,
            } => {
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

        let event: AriEvent = serde_json::from_str(json).expect("dtmf received should deserialize");

        match event {
            AriEvent::ChannelDtmfReceived {
                channel,
                digit,
                duration_ms,
            } => {
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

        let event: AriEvent =
            serde_json::from_str(json).expect("minimal stasis start should deserialize");

        match event {
            AriEvent::StasisStart {
                channel,
                args,
                replace_channel,
            } => {
                assert_eq!(channel.id, "abc");
                assert!(args.is_empty());
                assert!(replace_channel.is_none());
            }
            other => panic!("expected StasisStart, got {other:?}"),
        }
    }

    #[test]
    fn deserialize_dial_event() {
        let json = r#"{
            "type": "Dial",
            "peer": {
                "id": "peer-1",
                "name": "PJSIP/200-00000002",
                "state": "Ring"
            },
            "caller": {
                "id": "caller-1",
                "name": "PJSIP/100-00000001",
                "state": "Up"
            },
            "dialstatus": "RINGING",
            "dialstring": "200"
        }"#;
        let event: AriEvent = serde_json::from_str(json).expect("dial should deserialize");
        match event {
            AriEvent::Dial {
                peer,
                caller,
                dialstatus,
                ..
            } => {
                assert_eq!(peer.id, "peer-1");
                assert!(caller.is_some());
                assert_eq!(dialstatus, "RINGING");
            }
            other => panic!("expected Dial, got {other:?}"),
        }
    }

    #[test]
    fn deserialize_channel_hold_event() {
        let json = r#"{
            "type": "ChannelHold",
            "channel": {
                "id": "hold-1",
                "name": "PJSIP/100-00000001",
                "state": "Up"
            },
            "musicclass": "default"
        }"#;
        let event: AriEvent = serde_json::from_str(json).expect("hold should deserialize");
        match event {
            AriEvent::ChannelHold {
                channel,
                musicclass,
            } => {
                assert_eq!(channel.id, "hold-1");
                assert_eq!(musicclass.as_deref(), Some("default"));
            }
            other => panic!("expected ChannelHold, got {other:?}"),
        }
    }

    #[test]
    fn deserialize_bridge_blind_transfer() {
        let json = r#"{
            "type": "BridgeBlindTransfer",
            "channel": {
                "id": "xfer-1",
                "name": "PJSIP/100-00000001",
                "state": "Up"
            },
            "exten": "200",
            "context": "default",
            "result": "Success",
            "is_external": false
        }"#;
        let event: AriEvent =
            serde_json::from_str(json).expect("blind transfer should deserialize");
        match event {
            AriEvent::BridgeBlindTransfer {
                channel,
                exten,
                context,
                result,
                is_external,
                ..
            } => {
                assert_eq!(channel.id, "xfer-1");
                assert_eq!(exten, "200");
                assert_eq!(context, "default");
                assert_eq!(result, "Success");
                assert!(!is_external);
            }
            other => panic!("expected BridgeBlindTransfer, got {other:?}"),
        }
    }

    #[test]
    fn deserialize_peer_status_change() {
        let json = r#"{
            "type": "PeerStatusChange",
            "endpoint": {
                "technology": "PJSIP",
                "resource": "100",
                "state": "online"
            },
            "peer": {
                "peer_status": "Reachable",
                "address": "192.168.1.100",
                "port": "5060"
            }
        }"#;
        let event: AriEvent = serde_json::from_str(json).expect("peer status should deserialize");
        match event {
            AriEvent::PeerStatusChange { endpoint, peer } => {
                assert_eq!(endpoint.resource, "100");
                assert_eq!(peer.peer_status, "Reachable");
            }
            other => panic!("expected PeerStatusChange, got {other:?}"),
        }
    }

    #[test]
    fn deserialize_contact_status_change() {
        let json = r#"{
            "type": "ContactStatusChange",
            "contact_info": {
                "uri": "sip:100@192.168.1.100:5060",
                "contact_status": "Reachable",
                "aor": "100"
            },
            "endpoint": {
                "technology": "PJSIP",
                "resource": "100"
            }
        }"#;
        let event: AriEvent =
            serde_json::from_str(json).expect("contact status should deserialize");
        match event {
            AriEvent::ContactStatusChange {
                contact_info,
                endpoint,
            } => {
                assert_eq!(contact_info.aor, "100");
                assert_eq!(endpoint.resource, "100");
            }
            other => panic!("expected ContactStatusChange, got {other:?}"),
        }
    }

    #[test]
    fn deserialize_device_state_changed() {
        let json = r#"{
            "type": "DeviceStateChanged",
            "device_state": {
                "name": "PJSIP/100",
                "state": "INUSE"
            }
        }"#;
        let event: AriEvent = serde_json::from_str(json).expect("device state should deserialize");
        match event {
            AriEvent::DeviceStateChanged { device_state } => {
                assert_eq!(device_state.name, "PJSIP/100");
                assert_eq!(device_state.state, "INUSE");
            }
            other => panic!("expected DeviceStateChanged, got {other:?}"),
        }
    }

    #[test]
    fn deserialize_playback_continuing() {
        let json = r#"{
            "type": "PlaybackContinuing",
            "playback": {
                "id": "pb-1",
                "media_uri": "sound:hello-world",
                "state": "playing",
                "target_uri": "channel:abc"
            }
        }"#;
        let event: AriEvent =
            serde_json::from_str(json).expect("playback continuing should deserialize");
        assert!(matches!(event, AriEvent::PlaybackContinuing { .. }));
    }

    #[test]
    fn deserialize_recording_failed() {
        let json = r#"{
            "type": "RecordingFailed",
            "recording": {
                "name": "rec-1",
                "format": "wav",
                "state": "failed",
                "target_uri": "channel:abc"
            }
        }"#;
        let event: AriEvent =
            serde_json::from_str(json).expect("recording failed should deserialize");
        assert!(matches!(event, AriEvent::RecordingFailed { .. }));
    }

    #[test]
    fn deserialize_ari_message_with_metadata() {
        let json = r#"{
            "type": "StasisStart",
            "application": "my-app",
            "timestamp": "2024-01-15T10:30:00.000+0000",
            "asterisk_id": "00:11:22:33:44:55",
            "channel": {
                "id": "1234.5",
                "name": "PJSIP/alice-00000001",
                "state": "Ring"
            }
        }"#;
        let msg: AriMessage = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(msg.application, "my-app");
        assert_eq!(msg.timestamp, "2024-01-15T10:30:00.000+0000");
        assert_eq!(msg.asterisk_id.as_deref(), Some("00:11:22:33:44:55"));
        assert!(matches!(msg.event, AriEvent::StasisStart { .. }));
    }

    #[test]
    fn deserialize_ari_message_without_metadata() {
        let json = r#"{
            "type": "StasisEnd",
            "channel": {
                "id": "1234.5",
                "name": "PJSIP/alice-00000001",
                "state": "Up"
            }
        }"#;
        let msg: AriMessage = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(msg.application, "");
        assert_eq!(msg.timestamp, "");
        assert!(msg.asterisk_id.is_none());
        assert!(matches!(msg.event, AriEvent::StasisEnd { .. }));
    }
}
