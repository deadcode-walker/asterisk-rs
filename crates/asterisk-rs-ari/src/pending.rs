use std::sync::atomic::{AtomicU64, Ordering};

use crate::client::AriClient;
use crate::error::Result;
use crate::event::{AriEvent, AriMessage, Bridge, Channel};
use crate::resources::bridge::BridgeHandle;
use crate::resources::channel::{ChannelHandle, OriginateParams};
use asterisk_rs_core::event::FilteredSubscription;

static PENDING_COUNTER: AtomicU64 = AtomicU64::new(1);

fn generate_pending_id(prefix: &str) -> String {
    let id = PENDING_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-pending-{id}")
}

/// returns true if the event involves the given channel id
fn event_matches_channel_id(event: &AriEvent, id: &str) -> bool {
    match event {
        AriEvent::StasisStart { channel, .. }
        | AriEvent::StasisEnd { channel }
        | AriEvent::ChannelCreated { channel }
        | AriEvent::ChannelDestroyed { channel, .. }
        | AriEvent::ChannelStateChange { channel }
        | AriEvent::ChannelDtmfReceived { channel, .. }
        | AriEvent::ChannelHangupRequest { channel }
        | AriEvent::ChannelCallerId { channel, .. }
        | AriEvent::ChannelConnectedLine { channel }
        | AriEvent::ChannelDialplan { channel, .. }
        | AriEvent::ChannelHold { channel, .. }
        | AriEvent::ChannelUnhold { channel }
        | AriEvent::ChannelTalkingStarted { channel }
        | AriEvent::ChannelTalkingFinished { channel, .. }
        | AriEvent::ChannelToneDetected { channel }
        | AriEvent::ChannelTransfer { channel, .. }
        | AriEvent::ChannelEnteredBridge { channel, .. }
        | AriEvent::ChannelLeftBridge { channel, .. }
        | AriEvent::ApplicationMoveFailed { channel, .. } => channel.id == id,
        // dial events involve multiple participants; any could be the watched channel
        AriEvent::Dial {
            peer,
            caller,
            forwarded,
            ..
        } => {
            peer.id == id
                || caller.as_ref().is_some_and(|c| c.id == id)
                || forwarded.as_ref().is_some_and(|c| c.id == id)
        }
        // blind transfer involves the transferring channel and the transferee
        AriEvent::BridgeBlindTransfer {
            channel,
            transferee,
            replace_channel,
            ..
        } => {
            channel.id == id
                || transferee.as_ref().is_some_and(|c| c.id == id)
                || replace_channel.as_ref().is_some_and(|c| c.id == id)
        }
        _ => false,
    }
}

/// returns true if the event involves the given bridge id
fn event_matches_bridge_id(event: &AriEvent, id: &str) -> bool {
    match event {
        AriEvent::BridgeCreated { bridge }
        | AriEvent::BridgeDestroyed { bridge }
        | AriEvent::ChannelEnteredBridge { bridge, .. }
        | AriEvent::ChannelLeftBridge { bridge, .. }
        | AriEvent::BridgeVideoSourceChanged { bridge, .. } => bridge.id == id,
        // both bridges checked: the merged-from bridge may be the one being watched
        AriEvent::BridgeMerged {
            bridge,
            bridge_from,
        } => bridge.id == id || bridge_from.id == id,
        // bridge is optional on blind transfer
        AriEvent::BridgeBlindTransfer { bridge, .. } => bridge.as_ref().is_some_and(|b| b.id == id),
        // attended transfer may route through several optional bridge legs
        AriEvent::BridgeAttendedTransfer {
            transferer_first_leg_bridge,
            transferer_second_leg_bridge,
            destination_threeway_bridge,
            ..
        } => {
            transferer_first_leg_bridge
                .as_ref()
                .is_some_and(|b| b.id == id)
                || transferer_second_leg_bridge
                    .as_ref()
                    .is_some_and(|b| b.id == id)
                || destination_threeway_bridge
                    .as_ref()
                    .is_some_and(|b| b.id == id)
        }
        _ => false,
    }
}

/// extracts the playback id from an event, if present
fn event_playback_id(event: &AriEvent) -> Option<&str> {
    match event {
        AriEvent::PlaybackStarted { playback }
        | AriEvent::PlaybackFinished { playback }
        | AriEvent::PlaybackContinuing { playback } => Some(&playback.id),
        _ => None,
    }
}

/// a channel that has been pre-registered for events but not yet created.
///
/// solves the race condition between originate and event subscription:
/// the event filter is active before the originate call, so no events
/// are missed.
#[derive(Debug)]
pub struct PendingChannel {
    id: String,
    client: AriClient,
    events: FilteredSubscription<AriMessage>,
}

impl PendingChannel {
    /// create a new pending channel with a pre-generated ID
    pub(crate) fn new(client: AriClient) -> Self {
        let id = generate_pending_id("channel");
        let filter_id = id.clone();
        let events =
            client.subscribe_filtered(move |msg| event_matches_channel_id(&msg.event, &filter_id));

        Self { id, client, events }
    }

    /// the pre-generated channel ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// originate a channel using the pre-generated ID
    ///
    /// sets the `channel_id` field on params before sending the request.
    /// returns a ChannelHandle and the pre-subscribed event stream.
    pub async fn originate(
        self,
        mut params: OriginateParams,
    ) -> Result<(ChannelHandle, FilteredSubscription<AriMessage>)> {
        params.channel_id = Some(self.id.clone());
        let channel: Channel = self.client.post("/channels", &params).await?;
        let handle = ChannelHandle::new(channel.id, self.client);
        Ok((handle, self.events))
    }

    /// access the pre-subscribed event stream
    ///
    /// events matching this channel's ID are buffered from the moment
    /// PendingChannel was created
    pub fn events_mut(&mut self) -> &mut FilteredSubscription<AriMessage> {
        &mut self.events
    }
}

/// a bridge that has been pre-registered for events but not yet created
#[derive(Debug)]
pub struct PendingBridge {
    id: String,
    client: AriClient,
    events: FilteredSubscription<AriMessage>,
}

impl PendingBridge {
    /// create a new pending bridge with a pre-generated ID
    pub(crate) fn new(client: AriClient) -> Self {
        let id = generate_pending_id("bridge");
        let filter_id = id.clone();
        let events =
            client.subscribe_filtered(move |msg| event_matches_bridge_id(&msg.event, &filter_id));

        Self { id, client, events }
    }

    /// the pre-generated bridge ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// create the bridge with the pre-generated ID
    ///
    /// returns a BridgeHandle and the pre-subscribed event stream.
    pub async fn create(
        self,
        bridge_type: &str,
    ) -> Result<(BridgeHandle, FilteredSubscription<AriMessage>)> {
        let bridge: Bridge = self
            .client
            .post(
                "/bridges",
                &serde_json::json!({ "bridgeId": self.id, "type": bridge_type }),
            )
            .await?;
        let handle = BridgeHandle::new(bridge.id, self.client);
        Ok((handle, self.events))
    }

    /// access the pre-subscribed event stream
    pub fn events_mut(&mut self) -> &mut FilteredSubscription<AriMessage> {
        &mut self.events
    }
}

/// a playback that has been pre-registered for events but not yet started
#[derive(Debug)]
pub struct PendingPlayback {
    id: String,
    events: FilteredSubscription<AriMessage>,
}

impl PendingPlayback {
    /// create a new pending playback with a pre-generated ID
    pub(crate) fn new(client: &AriClient) -> Self {
        let id = generate_pending_id("playback");
        let filter_id = id.clone();
        let events = client.subscribe_filtered(move |msg| {
            event_playback_id(&msg.event).is_some_and(|pb_id| pb_id == filter_id)
        });

        Self { id, events }
    }

    /// the pre-generated playback ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// consume and return the pre-subscribed event stream
    pub fn into_events(self) -> FilteredSubscription<AriMessage> {
        self.events
    }
}
