//! call correlation engine — tracks AMI events by UniqueID into call lifecycle objects.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, watch};

use crate::event::AmiEvent;
use asterisk_rs_core::event::EventSubscription;

/// a fully resolved call with all collected events
#[derive(Debug, Clone)]
pub struct CompletedCall {
    /// channel name at creation
    pub channel: String,
    /// per-channel unique identifier
    pub unique_id: String,
    /// links bridged channels together
    pub linked_id: String,
    /// when the channel was created
    pub start_time: Instant,
    /// when the channel hung up
    pub end_time: Instant,
    /// total call duration
    pub duration: Duration,
    /// hangup cause code
    pub cause: u32,
    /// hangup cause description
    pub cause_txt: String,
    /// all events collected during this call's lifetime
    pub events: Vec<AmiEvent>,
}

/// tracks an in-progress call
struct ActiveCall {
    channel: String,
    unique_id: String,
    linked_id: String,
    start_time: Instant,
    events: Vec<AmiEvent>,
}

/// correlates AMI events by UniqueID into complete call records
///
/// spawns a background task that consumes events from an EventSubscription,
/// tracks active calls, and emits CompletedCall records when channels hang up.
pub struct CallTracker {
    shutdown_tx: watch::Sender<bool>,
    task_handle: tokio::task::JoinHandle<()>,
}

impl std::fmt::Debug for CallTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallTracker").finish_non_exhaustive()
    }
}

impl CallTracker {
    /// create a tracker that consumes events and produces completed call records
    pub fn new(subscription: EventSubscription<AmiEvent>) -> (Self, mpsc::Receiver<CompletedCall>) {
        let (completed_tx, completed_rx) = mpsc::channel(256);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let task_handle = tokio::spawn(track_loop(subscription, completed_tx, shutdown_rx));

        let tracker = Self {
            shutdown_tx,
            task_handle,
        };

        (tracker, completed_rx)
    }

    /// stop the background tracking task
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
        self.task_handle.abort();
    }
}

impl Drop for CallTracker {
    fn drop(&mut self) {
        self.shutdown();
    }
}

async fn track_loop(
    mut subscription: EventSubscription<AmiEvent>,
    completed_tx: mpsc::Sender<CompletedCall>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let mut active: HashMap<String, ActiveCall> = HashMap::new();

    loop {
        tokio::select! {
            event = subscription.recv() => {
                let Some(event) = event else { break };
                handle_event(&mut active, &completed_tx, event).await;
            }
            _ = shutdown_rx.changed() => {
                break;
            }
        }
    }
}

async fn handle_event(
    active: &mut HashMap<String, ActiveCall>,
    completed_tx: &mpsc::Sender<CompletedCall>,
    event: AmiEvent,
) {
    // handle new channel creation
    if let AmiEvent::NewChannel {
        ref channel,
        ref unique_id,
        ref linked_id,
        ..
    } = event
    {
        let call = ActiveCall {
            channel: channel.clone(),
            unique_id: unique_id.clone(),
            linked_id: linked_id.clone(),
            start_time: Instant::now(),
            events: vec![event],
        };
        active.insert(call.unique_id.clone(), call);
        return;
    }

    // handle hangup — finalize the call
    if let AmiEvent::Hangup {
        ref unique_id,
        cause,
        ref cause_txt,
        ..
    } = event
    {
        if let Some(mut call) = active.remove(unique_id.as_str()) {
            let end_time = Instant::now();
            let cause_txt = cause_txt.clone();
            call.events.push(event);
            let completed = CompletedCall {
                channel: call.channel,
                unique_id: call.unique_id,
                linked_id: call.linked_id,
                start_time: call.start_time,
                end_time,
                duration: end_time.duration_since(call.start_time),
                cause,
                cause_txt,
                events: call.events,
            };
            // receiver may have been dropped — ignore send errors
            let _ = completed_tx.send(completed).await;
        }
        return;
    }

    // for all other events, append to the matching active call if tracked
    if let Some(uid) = extract_unique_id(&event) {
        if let Some(call) = active.get_mut(uid) {
            call.events.push(event);
        }
    }
}

/// extract the unique_id field from an event, if present
fn extract_unique_id(event: &AmiEvent) -> Option<&str> {
    match event {
        // variants with a unique_id: String field
        AmiEvent::NewChannel { unique_id, .. }
        | AmiEvent::Hangup { unique_id, .. }
        | AmiEvent::Newstate { unique_id, .. }
        | AmiEvent::DialBegin { unique_id, .. }
        | AmiEvent::DialEnd { unique_id, .. }
        | AmiEvent::DtmfBegin { unique_id, .. }
        | AmiEvent::DtmfEnd { unique_id, .. }
        | AmiEvent::BridgeEnter { unique_id, .. }
        | AmiEvent::BridgeLeave { unique_id, .. }
        | AmiEvent::VarSet { unique_id, .. }
        | AmiEvent::Hold { unique_id, .. }
        | AmiEvent::Unhold { unique_id, .. }
        | AmiEvent::HangupRequest { unique_id, .. }
        | AmiEvent::SoftHangupRequest { unique_id, .. }
        | AmiEvent::NewExten { unique_id, .. }
        | AmiEvent::NewCallerid { unique_id, .. }
        | AmiEvent::NewConnectedLine { unique_id, .. }
        | AmiEvent::NewAccountCode { unique_id, .. }
        | AmiEvent::Rename { unique_id, .. }
        | AmiEvent::OriginateResponse { unique_id, .. }
        | AmiEvent::DialState { unique_id, .. }
        | AmiEvent::Flash { unique_id, .. }
        | AmiEvent::Wink { unique_id, .. }
        | AmiEvent::BridgeInfoChannel { unique_id, .. }
        | AmiEvent::LocalBridge { unique_id, .. }
        | AmiEvent::LocalOptimizationBegin { unique_id, .. }
        | AmiEvent::LocalOptimizationEnd { unique_id, .. }
        | AmiEvent::Cdr { unique_id, .. }
        | AmiEvent::Cel { unique_id, .. }
        | AmiEvent::QueueCallerAbandon { unique_id, .. }
        | AmiEvent::QueueCallerJoin { unique_id, .. }
        | AmiEvent::QueueCallerLeave { unique_id, .. }
        | AmiEvent::QueueEntry { unique_id, .. }
        | AmiEvent::AgentCalled { unique_id, .. }
        | AmiEvent::AgentConnect { unique_id, .. }
        | AmiEvent::AgentComplete { unique_id, .. }
        | AmiEvent::AgentDump { unique_id, .. }
        | AmiEvent::AgentLogin { unique_id, .. }
        | AmiEvent::AgentRingNoAnswer { unique_id, .. }
        | AmiEvent::ConfbridgeJoin { unique_id, .. }
        | AmiEvent::ConfbridgeLeave { unique_id, .. }
        | AmiEvent::ConfbridgeList { unique_id, .. }
        | AmiEvent::ConfbridgeMute { unique_id, .. }
        | AmiEvent::ConfbridgeUnmute { unique_id, .. }
        | AmiEvent::ConfbridgeTalking { unique_id, .. }
        | AmiEvent::MixMonitorStart { unique_id, .. }
        | AmiEvent::MixMonitorStop { unique_id, .. }
        | AmiEvent::MixMonitorMute { unique_id, .. }
        | AmiEvent::MusicOnHoldStart { unique_id, .. }
        | AmiEvent::MusicOnHoldStop { unique_id, .. }
        | AmiEvent::ParkedCall { unique_id, .. }
        | AmiEvent::ParkedCallGiveUp { unique_id, .. }
        | AmiEvent::ParkedCallTimeOut { unique_id, .. }
        | AmiEvent::ParkedCallSwap { unique_id, .. }
        | AmiEvent::UnParkedCall { unique_id, .. }
        | AmiEvent::Pickup { unique_id, .. }
        | AmiEvent::ChanSpyStart { unique_id, .. }
        | AmiEvent::ChanSpyStop { unique_id, .. }
        | AmiEvent::ChannelTalkingStart { unique_id, .. }
        | AmiEvent::ChannelTalkingStop { unique_id, .. }
        | AmiEvent::RTCPReceived { unique_id, .. }
        | AmiEvent::RTCPSent { unique_id, .. }
        | AmiEvent::AsyncAGIStart { unique_id, .. }
        | AmiEvent::AsyncAGIExec { unique_id, .. }
        | AmiEvent::AsyncAGIEnd { unique_id, .. }
        | AmiEvent::AGIExecStart { unique_id, .. }
        | AmiEvent::AGIExecEnd { unique_id, .. }
        | AmiEvent::HangupHandlerPush { unique_id, .. }
        | AmiEvent::HangupHandlerPop { unique_id, .. }
        | AmiEvent::HangupHandlerRun { unique_id, .. }
        | AmiEvent::Status { unique_id, .. }
        | AmiEvent::CoreShowChannel { unique_id, .. }
        | AmiEvent::AocD { unique_id, .. }
        | AmiEvent::AocE { unique_id, .. }
        | AmiEvent::AocS { unique_id, .. }
        | AmiEvent::FAXStatus { unique_id, .. }
        | AmiEvent::ReceiveFAX { unique_id, .. }
        | AmiEvent::SendFAX { unique_id, .. }
        | AmiEvent::MeetmeJoin { unique_id, .. }
        | AmiEvent::MeetmeLeave { unique_id, .. }
        | AmiEvent::MeetmeMute { unique_id, .. }
        | AmiEvent::MeetmeTalking { unique_id, .. }
        | AmiEvent::MeetmeTalkRequest { unique_id, .. }
        | AmiEvent::MeetmeList { unique_id, .. }
        | AmiEvent::MiniVoiceMail { unique_id, .. }
        | AmiEvent::FAXSession { unique_id, .. }
        | AmiEvent::MCID { unique_id, .. } => Some(unique_id.as_str()),

        // transferer_unique_id — not named unique_id but still useful
        AmiEvent::AttendedTransfer {
            transferer_unique_id,
            ..
        } => Some(transferer_unique_id.as_str()),
        AmiEvent::BlindTransfer {
            transferer_unique_id,
            ..
        } => Some(transferer_unique_id.as_str()),

        // unique_id is Option<String>
        AmiEvent::UserEvent { unique_id, .. } => unique_id.as_deref(),
        AmiEvent::DAHDIChannel { unique_id, .. } => unique_id.as_deref(),

        // variants without unique_id
        AmiEvent::FullyBooted { .. }
        | AmiEvent::PeerStatus { .. }
        | AmiEvent::BridgeCreate { .. }
        | AmiEvent::BridgeDestroy { .. }
        | AmiEvent::BridgeMerge { .. }
        | AmiEvent::BridgeInfoComplete { .. }
        | AmiEvent::BridgeVideoSourceUpdate { .. }
        | AmiEvent::QueueMemberAdded { .. }
        | AmiEvent::QueueMemberRemoved { .. }
        | AmiEvent::QueueMemberPause { .. }
        | AmiEvent::QueueMemberStatus { .. }
        | AmiEvent::QueueMemberPenalty { .. }
        | AmiEvent::QueueMemberRinginuse { .. }
        | AmiEvent::QueueParams { .. }
        | AmiEvent::AgentLogoff { .. }
        | AmiEvent::Agents { .. }
        | AmiEvent::AgentsComplete
        | AmiEvent::ConfbridgeStart { .. }
        | AmiEvent::ConfbridgeEnd { .. }
        | AmiEvent::ConfbridgeRecord { .. }
        | AmiEvent::ConfbridgeStopRecord { .. }
        | AmiEvent::ConfbridgeListRooms { .. }
        | AmiEvent::DeviceStateChange { .. }
        | AmiEvent::ExtensionStatus { .. }
        | AmiEvent::PresenceStateChange { .. }
        | AmiEvent::PresenceStatus { .. }
        | AmiEvent::ContactStatus { .. }
        | AmiEvent::Registry { .. }
        | AmiEvent::MessageWaiting { .. }
        | AmiEvent::VoicemailPasswordChange { .. }
        | AmiEvent::FailedACL { .. }
        | AmiEvent::InvalidAccountID { .. }
        | AmiEvent::InvalidPassword { .. }
        | AmiEvent::ChallengeResponseFailed { .. }
        | AmiEvent::ChallengeSent { .. }
        | AmiEvent::SuccessfulAuth { .. }
        | AmiEvent::SessionLimit { .. }
        | AmiEvent::UnexpectedAddress { .. }
        | AmiEvent::RequestBadFormat { .. }
        | AmiEvent::RequestNotAllowed { .. }
        | AmiEvent::RequestNotSupported { .. }
        | AmiEvent::InvalidTransport { .. }
        | AmiEvent::AuthMethodNotAllowed { .. }
        | AmiEvent::Shutdown { .. }
        | AmiEvent::Reload { .. }
        | AmiEvent::Load { .. }
        | AmiEvent::Unload { .. }
        | AmiEvent::LogChannel { .. }
        | AmiEvent::LoadAverageLimit
        | AmiEvent::MemoryLimit
        | AmiEvent::StatusComplete { .. }
        | AmiEvent::CoreShowChannelsComplete { .. }
        | AmiEvent::CoreShowChannelMapComplete
        | AmiEvent::Alarm { .. }
        | AmiEvent::AlarmClear { .. }
        | AmiEvent::SpanAlarm { .. }
        | AmiEvent::SpanAlarmClear { .. }
        | AmiEvent::MeetmeEnd { .. }
        | AmiEvent::MeetmeListRooms { .. }
        | AmiEvent::DeviceStateListComplete { .. }
        | AmiEvent::ExtensionStateListComplete { .. }
        | AmiEvent::PresenceStateListComplete { .. }
        | AmiEvent::AorDetail { .. }
        | AmiEvent::AorList { .. }
        | AmiEvent::AorListComplete { .. }
        | AmiEvent::AuthDetail { .. }
        | AmiEvent::AuthList { .. }
        | AmiEvent::AuthListComplete { .. }
        | AmiEvent::ContactList { .. }
        | AmiEvent::ContactListComplete { .. }
        | AmiEvent::ContactStatusDetail { .. }
        | AmiEvent::EndpointDetail { .. }
        | AmiEvent::EndpointDetailComplete { .. }
        | AmiEvent::EndpointList { .. }
        | AmiEvent::EndpointListComplete { .. }
        | AmiEvent::IdentifyDetail { .. }
        | AmiEvent::TransportDetail { .. }
        | AmiEvent::ResourceListDetail { .. }
        | AmiEvent::InboundRegistrationDetail { .. }
        | AmiEvent::OutboundRegistrationDetail { .. }
        | AmiEvent::InboundSubscriptionDetail { .. }
        | AmiEvent::OutboundSubscriptionDetail { .. }
        | AmiEvent::MWIGet { .. }
        | AmiEvent::MWIGetComplete { .. }
        | AmiEvent::FAXSessionsEntry { .. }
        | AmiEvent::FAXSessionsComplete { .. }
        | AmiEvent::FAXStats { .. }
        | AmiEvent::DNDState { .. }
        | AmiEvent::DeadlockStart
        | AmiEvent::Unknown { .. } => None,
    }
}
