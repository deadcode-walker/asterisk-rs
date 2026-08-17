//! Typed ARI events deserialized from WebSocket JSON.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _, ser::Error as _};

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

impl AriEvent {
    /// Return every channel ID carried by this event.
    ///
    /// Multi-party and nested transfer events may return more than one ID. The
    /// match is intentionally exhaustive so newly modeled events must make an
    /// explicit choice about channel identity.
    pub fn channel_ids(&self) -> Vec<&str> {
        fn push_channel<'a>(ids: &mut Vec<&'a str>, channel: &'a Channel) {
            ids.push(channel.id.as_str());
        }
        fn push_optional_channel<'a>(ids: &mut Vec<&'a str>, channel: &'a Option<Channel>) {
            if let Some(channel) = channel {
                push_channel(ids, channel);
            }
        }
        let mut ids = Vec::new();
        match self {
            Self::StasisStart {
                channel: primary,
                replace_channel,
                ..
            } => {
                push_channel(&mut ids, primary);
                push_optional_channel(&mut ids, replace_channel);
            }
            Self::StasisEnd { channel: value }
            | Self::ChannelCreated { channel: value }
            | Self::ChannelDestroyed { channel: value, .. }
            | Self::ChannelStateChange { channel: value }
            | Self::ChannelDtmfReceived { channel: value, .. }
            | Self::ChannelHangupRequest { channel: value }
            | Self::ChannelCallerId { channel: value, .. }
            | Self::ChannelConnectedLine { channel: value }
            | Self::ChannelDialplan { channel: value, .. }
            | Self::ChannelHold { channel: value, .. }
            | Self::ChannelUnhold { channel: value }
            | Self::ChannelTalkingStarted { channel: value }
            | Self::ChannelTalkingFinished { channel: value, .. }
            | Self::ChannelToneDetected { channel: value }
            | Self::ChannelEnteredBridge { channel: value, .. }
            | Self::ChannelLeftBridge { channel: value, .. }
            | Self::ApplicationMoveFailed { channel: value, .. } => {
                push_channel(&mut ids, value);
            }
            Self::ChannelVarset { channel: value, .. }
            | Self::ChannelUserevent { channel: value, .. } => {
                push_optional_channel(&mut ids, value);
            }
            Self::ChannelTransfer {
                channel: primary,
                refer_to,
                referred_by,
                ..
            } => {
                push_channel(&mut ids, primary);
                if let Some(refer_to) = refer_to {
                    push_optional_channel(&mut ids, &refer_to.destination_channel);
                    push_optional_channel(&mut ids, &refer_to.connected_channel);
                }
                if let Some(referred_by) = referred_by {
                    push_channel(&mut ids, &referred_by.source_channel);
                    push_optional_channel(&mut ids, &referred_by.connected_channel);
                }
            }
            Self::Dial {
                peer,
                caller,
                forwarded,
                ..
            } => {
                push_channel(&mut ids, peer);
                push_optional_channel(&mut ids, caller);
                push_optional_channel(&mut ids, forwarded);
            }
            Self::BridgeAttendedTransfer {
                transferer_first_leg,
                transferer_second_leg,
                transferee,
                transfer_target,
                replace_channel,
                destination_link_first_leg,
                destination_link_second_leg,
                destination_threeway_channel,
                ..
            } => {
                push_channel(&mut ids, transferer_first_leg);
                push_channel(&mut ids, transferer_second_leg);
                for value in [
                    transferee,
                    transfer_target,
                    replace_channel,
                    destination_link_first_leg,
                    destination_link_second_leg,
                    destination_threeway_channel,
                ]
                .into_iter()
                .flatten()
                {
                    push_channel(&mut ids, value);
                }
            }
            Self::BridgeBlindTransfer {
                channel: primary,
                transferee,
                replace_channel,
                ..
            } => {
                push_channel(&mut ids, primary);
                push_optional_channel(&mut ids, transferee);
                push_optional_channel(&mut ids, replace_channel);
            }
            Self::ContactStatusChange { endpoint, .. }
            | Self::EndpointStateChange { endpoint }
            | Self::PeerStatusChange { endpoint, .. } => {
                ids.extend(endpoint.channel_ids.iter().map(String::as_str));
            }
            Self::BridgeCreated { .. }
            | Self::BridgeDestroyed { .. }
            | Self::PlaybackStarted { .. }
            | Self::PlaybackFinished { .. }
            | Self::RecordingStarted { .. }
            | Self::RecordingFinished { .. }
            | Self::BridgeMerged { .. }
            | Self::BridgeVideoSourceChanged { .. }
            | Self::DeviceStateChanged { .. }
            | Self::PlaybackContinuing { .. }
            | Self::RecordingFailed { .. }
            | Self::ApplicationRegistered {}
            | Self::ApplicationReplaced {}
            | Self::ApplicationUnregistered {}
            | Self::TextMessageReceived { .. }
            | Self::RESTResponse { .. }
            | Self::Unknown => {}
        }
        ids
    }

    /// Return every bridge ID carried by this event, including optional and
    /// merged-from bridge identities.
    pub fn bridge_ids(&self) -> Vec<&str> {
        fn push_bridge<'a>(ids: &mut Vec<&'a str>, bridge: &'a Bridge) {
            ids.push(bridge.id.as_str());
        }
        fn push_optional_bridge<'a>(ids: &mut Vec<&'a str>, bridge: &'a Option<Bridge>) {
            if let Some(bridge) = bridge {
                push_bridge(ids, bridge);
            }
        }
        let mut ids = Vec::new();
        match self {
            Self::BridgeCreated { bridge: value }
            | Self::BridgeDestroyed { bridge: value }
            | Self::ChannelEnteredBridge { bridge: value, .. }
            | Self::ChannelLeftBridge { bridge: value, .. }
            | Self::BridgeVideoSourceChanged { bridge: value, .. } => {
                push_bridge(&mut ids, value);
            }
            Self::ChannelUserevent { bridge: value, .. }
            | Self::BridgeBlindTransfer { bridge: value, .. } => {
                push_optional_bridge(&mut ids, value);
            }
            Self::ChannelTransfer {
                refer_to,
                referred_by,
                ..
            } => {
                if let Some(refer_to) = refer_to {
                    push_optional_bridge(&mut ids, &refer_to.bridge);
                }
                if let Some(referred_by) = referred_by {
                    push_optional_bridge(&mut ids, &referred_by.bridge);
                }
            }
            Self::BridgeAttendedTransfer {
                transferer_first_leg_bridge,
                transferer_second_leg_bridge,
                destination_bridge,
                destination_threeway_bridge,
                ..
            } => {
                push_optional_bridge(&mut ids, transferer_first_leg_bridge);
                push_optional_bridge(&mut ids, transferer_second_leg_bridge);
                if let Some(value) = destination_bridge {
                    ids.push(value.as_str());
                }
                push_optional_bridge(&mut ids, destination_threeway_bridge);
            }
            Self::BridgeMerged {
                bridge: value,
                bridge_from,
            } => {
                push_bridge(&mut ids, value);
                push_bridge(&mut ids, bridge_from);
            }
            Self::StasisStart { .. }
            | Self::StasisEnd { .. }
            | Self::ChannelCreated { .. }
            | Self::ChannelDestroyed { .. }
            | Self::ChannelStateChange { .. }
            | Self::ChannelDtmfReceived { .. }
            | Self::ChannelHangupRequest { .. }
            | Self::ChannelVarset { .. }
            | Self::PlaybackStarted { .. }
            | Self::PlaybackFinished { .. }
            | Self::RecordingStarted { .. }
            | Self::RecordingFinished { .. }
            | Self::ChannelCallerId { .. }
            | Self::ChannelConnectedLine { .. }
            | Self::ChannelDialplan { .. }
            | Self::ChannelHold { .. }
            | Self::ChannelUnhold { .. }
            | Self::ChannelTalkingStarted { .. }
            | Self::ChannelTalkingFinished { .. }
            | Self::ChannelToneDetected { .. }
            | Self::Dial { .. }
            | Self::ContactStatusChange { .. }
            | Self::DeviceStateChanged { .. }
            | Self::EndpointStateChange { .. }
            | Self::PeerStatusChange { .. }
            | Self::PlaybackContinuing { .. }
            | Self::RecordingFailed { .. }
            | Self::ApplicationMoveFailed { .. }
            | Self::ApplicationRegistered {}
            | Self::ApplicationReplaced {}
            | Self::ApplicationUnregistered {}
            | Self::TextMessageReceived { .. }
            | Self::RESTResponse { .. }
            | Self::Unknown => {}
        }
        ids
    }

    /// Return every playback ID carried by this event.
    pub fn playback_ids(&self) -> Vec<&str> {
        match self {
            Self::PlaybackStarted { playback }
            | Self::PlaybackFinished { playback }
            | Self::PlaybackContinuing { playback } => vec![playback.id.as_str()],
            Self::StasisStart { .. }
            | Self::StasisEnd { .. }
            | Self::ChannelCreated { .. }
            | Self::ChannelDestroyed { .. }
            | Self::ChannelStateChange { .. }
            | Self::ChannelDtmfReceived { .. }
            | Self::ChannelHangupRequest { .. }
            | Self::ChannelVarset { .. }
            | Self::BridgeCreated { .. }
            | Self::BridgeDestroyed { .. }
            | Self::ChannelEnteredBridge { .. }
            | Self::ChannelLeftBridge { .. }
            | Self::RecordingStarted { .. }
            | Self::RecordingFinished { .. }
            | Self::ChannelCallerId { .. }
            | Self::ChannelConnectedLine { .. }
            | Self::ChannelDialplan { .. }
            | Self::ChannelHold { .. }
            | Self::ChannelUnhold { .. }
            | Self::ChannelTalkingStarted { .. }
            | Self::ChannelTalkingFinished { .. }
            | Self::ChannelToneDetected { .. }
            | Self::ChannelTransfer { .. }
            | Self::ChannelUserevent { .. }
            | Self::Dial { .. }
            | Self::BridgeAttendedTransfer { .. }
            | Self::BridgeBlindTransfer { .. }
            | Self::BridgeMerged { .. }
            | Self::BridgeVideoSourceChanged { .. }
            | Self::ContactStatusChange { .. }
            | Self::DeviceStateChanged { .. }
            | Self::EndpointStateChange { .. }
            | Self::PeerStatusChange { .. }
            | Self::RecordingFailed { .. }
            | Self::ApplicationMoveFailed { .. }
            | Self::ApplicationRegistered {}
            | Self::ApplicationReplaced {}
            | Self::ApplicationUnregistered {}
            | Self::TextMessageReceived { .. }
            | Self::RESTResponse { .. }
            | Self::Unknown => Vec::new(),
        }
    }
}

/// an unrecognized ARI event retained for forward-compatible handling
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct UnknownAriEvent {
    /// upstream value of the event's `type` field
    pub event_type: String,
    /// original fields other than `type`, including common metadata when present
    pub payload: serde_json::Map<String, serde_json::Value>,
}

impl UnknownAriEvent {
    /// create retained data for an unrecognized upstream event
    pub fn new(
        event_type: impl Into<String>,
        payload: serde_json::Map<String, serde_json::Value>,
    ) -> Self {
        Self {
            event_type: event_type.into(),
            payload,
        }
    }
}

/// a complete ARI event with common metadata and typed payload
#[derive(Debug, Clone)]
pub struct AriMessage {
    /// the stasis application that received this event
    pub application: String,
    /// ISO 8601 timestamp when the event was created
    pub timestamp: String,
    /// unique id of the asterisk instance that generated this event
    pub asterisk_id: Option<String>,
    /// the typed event payload
    pub event: AriEvent,
    /// original type and payload when [`Self::event`] is [`AriEvent::Unknown`]
    pub unknown: Option<UnknownAriEvent>,
}

impl Serialize for AriMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut object = match (&self.event, &self.unknown) {
            (AriEvent::Unknown, Some(unknown)) => {
                if unknown.payload.contains_key("type") {
                    return Err(S::Error::custom(format_args!(
                        "unknown ARI event payload contains reserved field `type`"
                    )));
                }
                let metadata_matches =
                    |key: &str, expected: Option<&str>, null_matches: bool| match unknown
                        .payload
                        .get(key)
                    {
                        None => true,
                        Some(serde_json::Value::Null) => null_matches,
                        Some(serde_json::Value::String(value)) => expected == Some(value),
                        Some(_) => false,
                    };
                for (key, expected, null_matches) in [
                    (
                        "application",
                        Some(self.application.as_str()),
                        self.application.is_empty(),
                    ),
                    (
                        "timestamp",
                        Some(self.timestamp.as_str()),
                        self.timestamp.is_empty(),
                    ),
                    (
                        "asterisk_id",
                        self.asterisk_id.as_deref(),
                        self.asterisk_id.is_none(),
                    ),
                ] {
                    if !metadata_matches(key, expected, null_matches) {
                        return Err(S::Error::custom(format_args!(
                            "unknown ARI event payload contains contradictory reserved field `{key}`"
                        )));
                    }
                }
                let mut payload = unknown.payload.clone();
                payload.insert(
                    "type".to_owned(),
                    serde_json::Value::String(unknown.event_type.clone()),
                );
                payload
            }
            (AriEvent::Unknown, None) => {
                return Err(S::Error::custom(
                    "unknown ARI event is missing its retained type and payload",
                ));
            }
            (_, Some(_)) => {
                return Err(S::Error::custom(
                    "known ARI event cannot carry an unknown event payload",
                ));
            }
            (_, None) => serde_json::to_value(&self.event)
                .map_err(S::Error::custom)?
                .as_object()
                .cloned()
                .ok_or_else(|| S::Error::custom("ARI event did not serialize as an object"))?,
        };
        let is_unknown = matches!(self.event, AriEvent::Unknown);
        if !is_unknown || !self.application.is_empty() {
            object.insert(
                "application".to_owned(),
                serde_json::Value::String(self.application.clone()),
            );
        }
        if !is_unknown || !self.timestamp.is_empty() {
            object.insert(
                "timestamp".to_owned(),
                serde_json::Value::String(self.timestamp.clone()),
            );
        }
        match &self.asterisk_id {
            Some(asterisk_id) => {
                object.insert(
                    "asterisk_id".to_owned(),
                    serde_json::Value::String(asterisk_id.clone()),
                );
            }
            None if !is_unknown => {
                object.insert("asterisk_id".to_owned(), serde_json::Value::Null);
            }
            None => {}
        }
        object.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AriMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object()
            .ok_or_else(|| D::Error::custom("ARI event must be a JSON object"))?;
        let event_type = object
            .get("type")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| D::Error::missing_field("type"))?
            .to_owned();
        let event: AriEvent = serde_json::from_value(value.clone()).map_err(D::Error::custom)?;
        let is_unknown = matches!(event, AriEvent::Unknown);
        let application = string_field(object, "application", is_unknown)?;
        let timestamp = string_field(object, "timestamp", is_unknown)?;
        let asterisk_id = optional_string_field(object, "asterisk_id")?;
        let unknown = if is_unknown {
            let mut payload = object.clone();
            payload.remove("type");
            Some(UnknownAriEvent::new(event_type, payload))
        } else {
            None
        };

        Ok(Self {
            application,
            timestamp,
            asterisk_id,
            event,
            unknown,
        })
    }
}

fn string_field<E: serde::de::Error>(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
    allow_null: bool,
) -> Result<String, E> {
    match object.get(field) {
        None => Ok(String::new()),
        Some(serde_json::Value::Null) if allow_null => Ok(String::new()),
        Some(serde_json::Value::String(value)) => Ok(value.clone()),
        Some(_) => Err(E::custom(format_args!(
            "ARI field `{field}` must be a string"
        ))),
    }
}

fn optional_string_field<E: serde::de::Error>(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<String>, E> {
    match object.get(field) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(E::custom(format_args!(
            "ARI field `{field}` must be a string"
        ))),
    }
}

impl asterisk_rs_core::event::Event for AriMessage {}

/// contact info for PJSIP registration status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ContactInfo {
    pub uri: String,
    pub contact_status: String,
    pub aor: String,
    #[serde(default)]
    pub roundtrip_usec: Option<String>,
}

/// peer status information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
pub struct DeviceState {
    pub name: String,
    pub state: String,
}

/// text message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TextMessage {
    pub from: String,
    pub to: String,
    pub body: String,
    #[serde(default)]
    pub variables: serde_json::Map<String, serde_json::Value>,
}

/// refer-to information for channel transfers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ReferTo {
    #[serde(default)]
    pub requested_destination: serde_json::Value,
    #[serde(default)]
    pub destination_channel: Option<Channel>,
    #[serde(default)]
    pub connected_channel: Option<Channel>,
    #[serde(default)]
    pub bridge: Option<Bridge>,
}

/// referred-by information for channel transfers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ReferredBy {
    pub source_channel: Channel,
    #[serde(default)]
    pub connected_channel: Option<Channel>,
    #[serde(default)]
    pub bridge: Option<Bridge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Channel {
    pub id: String,
    #[serde(default)]
    pub protocol_id: String,
    pub name: String,
    pub state: String,
    #[serde(default)]
    pub caller: CallerId,
    #[serde(default)]
    pub connected: CallerId,
    #[serde(default)]
    pub accountcode: String,
    #[serde(default)]
    pub dialplan: DialplanCep,
    #[serde(default)]
    pub creationtime: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub channelvars: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub caller_rdnis: String,
    #[serde(default)]
    pub tenantid: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CallerId {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub number: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DialplanCep {
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub exten: String,
    #[serde(default)]
    pub priority: i64,
    #[serde(default)]
    pub app_name: String,
    #[serde(default)]
    pub app_data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Bridge {
    pub id: String,
    pub technology: String,
    pub bridge_type: String,
    #[serde(default)]
    pub bridge_class: String,
    #[serde(default)]
    pub creator: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub video_mode: Option<String>,
    #[serde(default)]
    pub video_source_id: Option<String>,
    #[serde(default)]
    pub creationtime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Playback {
    pub id: String,
    pub media_uri: String,
    #[serde(default)]
    pub next_media_uri: Option<String>,
    pub state: String,
    pub target_uri: String,
    #[serde(default)]
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LiveRecording {
    pub name: String,
    pub format: String,
    pub state: String,
    pub target_uri: String,
    #[serde(default)]
    pub duration: Option<i32>,
    #[serde(default)]
    pub talking_duration: Option<i32>,
    #[serde(default)]
    pub silence_duration: Option<i32>,
    #[serde(default)]
    pub cause: Option<String>,
}
