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
    /// channel entered a Stasis application
    StasisStart {
        channel: Channel,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        replace_channel: Option<Channel>,
    },
    /// channel left a Stasis application
    StasisEnd { channel: Channel },
    /// channel was created
    ChannelCreated { channel: Channel },
    /// channel was destroyed
    ChannelDestroyed {
        channel: Channel,
        cause: i32,
        cause_txt: String,
    },
    /// channel state changed
    ChannelStateChange { channel: Channel },
    /// DTMF digit received on channel
    ChannelDtmfReceived {
        channel: Channel,
        digit: String,
        duration_ms: u32,
    },
    /// hangup requested on channel
    ChannelHangupRequest { channel: Channel },
    /// channel variable set
    ChannelVarset {
        channel: Option<Channel>,
        variable: String,
        value: String,
    },
    /// bridge was created
    BridgeCreated { bridge: Bridge },
    /// bridge was destroyed
    BridgeDestroyed { bridge: Bridge },
    /// channel entered a bridge
    ChannelEnteredBridge { bridge: Bridge, channel: Channel },
    /// channel left a bridge
    ChannelLeftBridge { bridge: Bridge, channel: Channel },
    /// media playback started
    PlaybackStarted { playback: Playback },
    /// media playback finished
    PlaybackFinished { playback: Playback },
    /// recording started
    RecordingStarted { recording: LiveRecording },
    /// recording finished
    RecordingFinished { recording: LiveRecording },
    /// channel caller id changed
    ChannelCallerId {
        channel: Channel,
        caller_presentation: i32,
        caller_presentation_txt: String,
    },
    /// channel connected line changed
    ChannelConnectedLine { channel: Channel },
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
    ChannelUnhold { channel: Channel },
    /// channel talking started
    ChannelTalkingStarted { channel: Channel },
    /// channel talking finished
    ChannelTalkingFinished { channel: Channel, duration: i32 },
    /// tone detected on channel
    ChannelToneDetected { channel: Channel },
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
    BridgeMerged { bridge: Bridge, bridge_from: Bridge },
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
    DeviceStateChanged { device_state: DeviceState },
    /// endpoint state changed
    EndpointStateChange { endpoint: Endpoint },
    /// peer status changed
    PeerStatusChange { endpoint: Endpoint, peer: Peer },
    /// playback continuing to next media uri
    PlaybackContinuing { playback: Playback },
    /// recording failed
    RecordingFailed { recording: LiveRecording },
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
