//! typed AMI event types

use crate::codec::RawAmiMessage;
use serde::Serialize;
use std::collections::HashMap;

/// all known AMI event types
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
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
    FullyBooted { status: String },

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
    BridgeDestroy { bridge_unique_id: String },

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

    // ── core call flow ──
    /// channel variable set
    VarSet {
        channel: Option<String>,
        variable: String,
        value: String,
        unique_id: Option<String>,
    },

    /// channel placed on hold
    Hold {
        channel: String,
        unique_id: String,
        music_class: Option<String>,
    },

    /// channel taken off hold
    Unhold { channel: String, unique_id: String },

    /// hangup requested
    HangupRequest {
        channel: String,
        unique_id: String,
        cause: u32,
    },

    /// soft hangup requested
    SoftHangupRequest {
        channel: String,
        unique_id: String,
        cause: u32,
    },

    /// channel entered new dialplan extension
    NewExten {
        channel: String,
        context: String,
        extension: String,
        priority: u32,
        application: String,
        app_data: String,
        unique_id: String,
    },

    /// caller id changed
    NewCallerid {
        channel: String,
        caller_id_num: String,
        caller_id_name: String,
        unique_id: String,
        cid_calling_pres: String,
    },

    /// connected line info changed
    NewConnectedLine {
        channel: String,
        unique_id: String,
        connected_line_num: String,
        connected_line_name: String,
    },

    /// account code changed
    NewAccountCode {
        channel: String,
        unique_id: String,
        account_code: String,
        old_account_code: String,
    },

    /// channel renamed
    Rename {
        channel: String,
        new_name: String,
        unique_id: String,
    },

    /// originate result
    OriginateResponse {
        action_id: Option<String>,
        channel: String,
        unique_id: String,
        response: String,
        reason: String,
    },

    /// dial state changed
    DialState {
        channel: String,
        destination: String,
        dial_status: String,
        unique_id: String,
        dest_unique_id: String,
    },

    /// flash hook detected
    Flash { channel: String, unique_id: String },

    /// wink detected
    Wink { channel: String, unique_id: String },

    /// user-defined event
    UserEvent {
        channel: Option<String>,
        unique_id: Option<String>,
        user_event: String,
        headers: HashMap<String, String>,
    },

    // ── transfer ──
    /// attended transfer completed
    AttendedTransfer {
        result: String,
        transferer_channel: String,
        transferer_unique_id: String,
        transferee_channel: String,
        transferee_unique_id: String,
    },

    /// blind transfer completed
    BlindTransfer {
        result: String,
        transferer_channel: String,
        transferer_unique_id: String,
        extension: String,
        context: String,
    },

    // ── bridge extended ──
    /// two bridges merged
    BridgeMerge {
        bridge_unique_id: String,
        bridge_type: String,
        to_bridge_unique_id: String,
    },

    /// channel info in bridge listing
    BridgeInfoChannel {
        bridge_unique_id: String,
        channel: String,
        unique_id: String,
    },

    /// bridge info listing complete
    BridgeInfoComplete { bridge_unique_id: String },

    /// bridge video source changed
    BridgeVideoSourceUpdate {
        bridge_unique_id: String,
        bridge_video_source_unique_id: String,
    },

    // ── local channel ──
    /// local channel bridged
    LocalBridge {
        channel: String,
        unique_id: String,
        context: String,
        exten: String,
    },

    /// local optimization started
    LocalOptimizationBegin {
        channel: String,
        unique_id: String,
        source_unique_id: String,
        dest_unique_id: String,
    },

    /// local optimization ended
    LocalOptimizationEnd { channel: String, unique_id: String },

    // ── cdr / cel ──
    /// call detail record
    Cdr {
        channel: String,
        unique_id: String,
        destination: String,
        disposition: String,
        duration: u32,
        billable_seconds: u32,
        account_code: String,
        source: String,
        destination_context: String,
    },

    /// channel event logging
    Cel {
        channel: String,
        unique_id: String,
        event_name_cel: String,
        account_code: String,
        application_name: String,
        application_data: String,
    },

    // ── queue ──
    /// caller abandoned queue
    QueueCallerAbandon {
        channel: String,
        unique_id: String,
        queue: String,
        position: u32,
        original_position: u32,
        hold_time: u32,
    },

    /// caller joined queue
    QueueCallerJoin {
        channel: String,
        unique_id: String,
        queue: String,
        position: u32,
        count: u32,
    },

    /// caller left queue
    QueueCallerLeave {
        channel: String,
        unique_id: String,
        queue: String,
        position: u32,
        count: u32,
    },

    /// member added to queue
    QueueMemberAdded {
        queue: String,
        member_name: String,
        interface: String,
        state_interface: String,
        membership: String,
        penalty: u32,
        paused: String,
    },

    /// member removed from queue
    QueueMemberRemoved {
        queue: String,
        member_name: String,
        interface: String,
    },

    /// member paused/unpaused
    QueueMemberPause {
        queue: String,
        member_name: String,
        interface: String,
        paused: String,
        reason: String,
    },

    /// member status changed
    QueueMemberStatus {
        queue: String,
        member_name: String,
        interface: String,
        status: u32,
        paused: String,
        calls_taken: u32,
    },

    /// member penalty changed
    QueueMemberPenalty {
        queue: String,
        member_name: String,
        interface: String,
        penalty: u32,
    },

    /// member ringinuse changed
    QueueMemberRinginuse {
        queue: String,
        member_name: String,
        interface: String,
        ringinuse: String,
    },

    /// queue parameters
    QueueParams {
        queue: String,
        max: u32,
        strategy: String,
        calls: u32,
        holdtime: u32,
        talktime: u32,
        completed: u32,
        abandoned: u32,
    },

    /// queue entry
    QueueEntry {
        queue: String,
        position: u32,
        channel: String,
        unique_id: String,
        caller_id_num: String,
        caller_id_name: String,
        wait: u32,
    },

    // ── agent ──
    /// agent called from queue
    AgentCalled {
        channel: String,
        unique_id: String,
        queue: String,
        agent: String,
        destination_channel: String,
    },

    /// agent connected
    AgentConnect {
        channel: String,
        unique_id: String,
        queue: String,
        agent: String,
        hold_time: u32,
        bridge_unique_id: String,
    },

    /// agent completed call
    AgentComplete {
        channel: String,
        unique_id: String,
        queue: String,
        agent: String,
        hold_time: u32,
        talk_time: u32,
        reason: String,
    },

    /// agent dumped call
    AgentDump {
        channel: String,
        unique_id: String,
        queue: String,
        agent: String,
    },

    /// agent logged in
    AgentLogin {
        channel: String,
        unique_id: String,
        agent: String,
    },

    /// agent logged off
    AgentLogoff { agent: String, logintime: u32 },

    /// agent did not answer
    AgentRingNoAnswer {
        channel: String,
        unique_id: String,
        queue: String,
        agent: String,
        ring_time: u32,
    },

    /// agent list entry
    Agents {
        agent: String,
        name: String,
        status: String,
        channel: Option<String>,
    },

    /// agent list complete
    AgentsComplete,

    // ── conference ──
    /// confbridge started
    ConfbridgeStart {
        bridge_unique_id: String,
        conference: String,
    },

    /// confbridge ended
    ConfbridgeEnd {
        bridge_unique_id: String,
        conference: String,
    },

    /// user joined confbridge
    ConfbridgeJoin {
        bridge_unique_id: String,
        conference: String,
        channel: String,
        unique_id: String,
        admin: String,
    },

    /// user left confbridge
    ConfbridgeLeave {
        bridge_unique_id: String,
        conference: String,
        channel: String,
        unique_id: String,
    },

    /// confbridge list entry
    ConfbridgeList {
        bridge_unique_id: String,
        conference: String,
        channel: String,
        unique_id: String,
        admin: String,
        muted: String,
    },

    /// confbridge user muted
    ConfbridgeMute {
        bridge_unique_id: String,
        conference: String,
        channel: String,
        unique_id: String,
    },

    /// confbridge user unmuted
    ConfbridgeUnmute {
        bridge_unique_id: String,
        conference: String,
        channel: String,
        unique_id: String,
    },

    /// confbridge talking status changed
    ConfbridgeTalking {
        bridge_unique_id: String,
        conference: String,
        channel: String,
        unique_id: String,
        talking_status: String,
    },

    /// confbridge recording started
    ConfbridgeRecord {
        bridge_unique_id: String,
        conference: String,
    },

    /// confbridge recording stopped
    ConfbridgeStopRecord {
        bridge_unique_id: String,
        conference: String,
    },

    /// confbridge room list entry
    ConfbridgeListRooms {
        conference: String,
        parties: u32,
        marked: u32,
        locked: String,
    },

    // ── mixmonitor ──
    /// mixmonitor started
    MixMonitorStart { channel: String, unique_id: String },

    /// mixmonitor stopped
    MixMonitorStop { channel: String, unique_id: String },

    /// mixmonitor mute state changed
    MixMonitorMute {
        channel: String,
        unique_id: String,
        direction: String,
        state: String,
    },

    // ── music on hold ──
    /// music on hold started
    MusicOnHoldStart {
        channel: String,
        unique_id: String,
        class: String,
    },

    /// music on hold stopped
    MusicOnHoldStop { channel: String, unique_id: String },

    // ── parking ──
    /// call parked
    ParkedCall {
        channel: String,
        unique_id: String,
        parking_lot: String,
        parking_space: u32,
        parker_dial_string: String,
        timeout: u32,
    },

    /// parked caller gave up
    ParkedCallGiveUp {
        channel: String,
        unique_id: String,
        parking_lot: String,
        parking_space: u32,
    },

    /// parked call timed out
    ParkedCallTimeOut {
        channel: String,
        unique_id: String,
        parking_lot: String,
        parking_space: u32,
    },

    /// parked call swapped
    ParkedCallSwap {
        channel: String,
        unique_id: String,
        parking_lot: String,
        parking_space: u32,
        parker_channel: String,
    },

    /// parked call retrieved
    UnParkedCall {
        channel: String,
        unique_id: String,
        parking_lot: String,
        parking_space: u32,
        retriever_channel: String,
    },

    // ── pickup / spy ──
    /// call pickup
    Pickup {
        channel: String,
        unique_id: String,
        target_channel: String,
        target_unique_id: String,
    },

    /// channel spy started
    ChanSpyStart {
        channel: String,
        unique_id: String,
        spy_channel: String,
        spy_unique_id: String,
    },

    /// channel spy stopped
    ChanSpyStop {
        channel: String,
        unique_id: String,
        spy_channel: String,
        spy_unique_id: String,
    },

    // ── channel talking ──
    /// channel started talking
    ChannelTalkingStart { channel: String, unique_id: String },

    /// channel stopped talking
    ChannelTalkingStop {
        channel: String,
        unique_id: String,
        duration: u32,
    },

    // ── device / presence / extension state ──
    /// device state changed
    DeviceStateChange { device: String, state: String },

    /// extension status changed
    ExtensionStatus {
        exten: String,
        context: String,
        hint: String,
        status: u32,
        status_text: String,
    },

    /// presence state changed
    PresenceStateChange {
        presentity: String,
        status: String,
        subtype: String,
        message: String,
    },

    /// presence status
    PresenceStatus {
        presentity: String,
        status: String,
        subtype: String,
        message: String,
    },

    // ── pjsip / registration ──
    /// contact status changed
    ContactStatus {
        uri: String,
        contact_status: String,
        aor: String,
        endpoint_name: String,
    },

    /// registration status
    Registry {
        channel_type: String,
        domain: String,
        username: String,
        status: String,
        cause: String,
    },

    // ── message / voicemail ──
    /// message waiting indication
    MessageWaiting {
        mailbox: String,
        waiting: String,
        new_messages: u32,
        old_messages: u32,
    },

    /// voicemail password changed
    ///
    /// **security note**: `new_password` contains the plaintext voicemail
    /// PIN as sent by Asterisk. avoid logging or serializing this event
    /// without redaction.
    VoicemailPasswordChange {
        context: String,
        mailbox: String,
        new_password: String,
    },

    // ── rtcp ──
    /// rtcp packet received
    RTCPReceived {
        channel: String,
        unique_id: String,
        ssrc: String,
        pt: String,
        from: String,
    },

    /// rtcp packet sent
    RTCPSent {
        channel: String,
        unique_id: String,
        ssrc: String,
        pt: String,
        to: String,
    },

    // ── security ──
    /// acl check failed
    FailedACL {
        severity: String,
        service: String,
        account_id: String,
        remote_address: String,
    },

    /// invalid account id
    InvalidAccountID {
        severity: String,
        service: String,
        account_id: String,
        remote_address: String,
    },

    /// invalid password
    InvalidPassword {
        severity: String,
        service: String,
        account_id: String,
        remote_address: String,
    },

    /// challenge-response failed
    ChallengeResponseFailed {
        severity: String,
        service: String,
        account_id: String,
        remote_address: String,
    },

    /// challenge sent
    ChallengeSent {
        severity: String,
        service: String,
        account_id: String,
        remote_address: String,
    },

    /// authentication succeeded
    SuccessfulAuth {
        severity: String,
        service: String,
        account_id: String,
        remote_address: String,
    },

    /// session limit reached
    SessionLimit {
        severity: String,
        service: String,
        account_id: String,
        remote_address: String,
    },

    /// unexpected source address
    UnexpectedAddress {
        severity: String,
        service: String,
        account_id: String,
        remote_address: String,
    },

    /// bad request format
    RequestBadFormat {
        severity: String,
        service: String,
        account_id: String,
        remote_address: String,
    },

    /// request not allowed
    RequestNotAllowed {
        severity: String,
        service: String,
        account_id: String,
        remote_address: String,
    },

    /// request not supported
    RequestNotSupported {
        severity: String,
        service: String,
        account_id: String,
        remote_address: String,
    },

    /// invalid transport
    InvalidTransport {
        severity: String,
        service: String,
        account_id: String,
        remote_address: String,
    },

    /// auth method not allowed
    AuthMethodNotAllowed {
        severity: String,
        service: String,
        account_id: String,
        remote_address: String,
    },

    // ── system ──
    /// asterisk shutting down
    Shutdown {
        shutdown_status: String,
        restart: String,
    },

    /// module reloaded
    Reload { module: String, status: String },

    /// module loaded
    Load { module: String, status: String },

    /// module unloaded
    Unload { module: String, status: String },

    /// log channel toggled
    LogChannel {
        channel_log: String,
        enabled: String,
    },

    /// load average exceeded limit
    LoadAverageLimit,

    /// memory usage exceeded limit
    MemoryLimit,

    // ── async agi ──
    /// async agi session started
    AsyncAGIStart {
        channel: String,
        unique_id: String,
        env: String,
    },

    /// async agi command executed
    AsyncAGIExec {
        channel: String,
        unique_id: String,
        command_id: String,
        result: String,
    },

    /// async agi session ended
    AsyncAGIEnd { channel: String, unique_id: String },

    /// agi command execution started
    AGIExecStart {
        channel: String,
        unique_id: String,
        command: String,
        command_id: String,
    },

    /// agi command execution ended
    AGIExecEnd {
        channel: String,
        unique_id: String,
        command: String,
        command_id: String,
        result_code: String,
        result: String,
    },

    // ── hangup handlers ──
    /// hangup handler pushed
    HangupHandlerPush {
        channel: String,
        unique_id: String,
        handler: String,
    },

    /// hangup handler popped
    HangupHandlerPop {
        channel: String,
        unique_id: String,
        handler: String,
    },

    /// hangup handler running
    HangupHandlerRun {
        channel: String,
        unique_id: String,
        handler: String,
    },

    // ── core show / status ──
    /// channel status entry
    Status {
        channel: String,
        unique_id: String,
        channel_state: String,
        caller_id_num: String,
        caller_id_name: String,
        account_code: String,
        context: String,
        exten: String,
        priority: u32,
        seconds: u32,
        bridge_id: String,
        /// channel variables present on the channel at query time
        channel_variables: HashMap<String, String>,
    },

    /// status listing complete
    StatusComplete { items: u32 },

    /// core show channel entry
    CoreShowChannel {
        channel: String,
        unique_id: String,
        channel_state: String,
        caller_id_num: String,
        caller_id_name: String,
        application: String,
        application_data: String,
        duration: String,
        bridge_id: String,
        /// channel variables present on the channel at query time
        channel_variables: HashMap<String, String>,
    },

    /// core show channels complete
    CoreShowChannelsComplete { listed_channels: u32 },

    /// core show channel map complete
    CoreShowChannelMapComplete,

    // ── dahdi ──
    /// dahdi channel info
    DAHDIChannel {
        dahdi_channel: String,
        channel: Option<String>,
        unique_id: Option<String>,
    },

    /// dahdi alarm
    Alarm {
        alarm: String,
        channel_dahdi: String,
    },

    /// dahdi alarm cleared
    AlarmClear { channel_dahdi: String },

    /// dahdi span alarm
    SpanAlarm { span: u32, alarm: String },

    /// dahdi span alarm cleared
    SpanAlarmClear { span: u32 },

    // ── aoc ──
    /// advice of charge — during call
    AocD {
        channel: String,
        unique_id: String,
        charge_type: String,
    },

    /// advice of charge — end of call
    AocE {
        channel: String,
        unique_id: String,
        charge_type: String,
    },

    /// advice of charge — setup
    AocS { channel: String, unique_id: String },

    // ── fax ──
    /// fax status update
    FAXStatus {
        channel: String,
        unique_id: String,
        operation: String,
        status: String,
        local_station_id: String,
        filename: String,
    },

    /// fax received
    ReceiveFAX {
        channel: String,
        unique_id: String,
        local_station_id: String,
        remote_station_id: String,
        pages_transferred: u32,
        resolution: String,
        filename: String,
    },

    /// fax sent
    SendFAX {
        channel: String,
        unique_id: String,
        local_station_id: String,
        remote_station_id: String,
        pages_transferred: u32,
        resolution: String,
        filename: String,
    },

    // ── meetme ──
    /// meetme user joined
    MeetmeJoin {
        meetme: String,
        user_num: String,
        channel: String,
        unique_id: String,
    },

    /// meetme user left
    MeetmeLeave {
        meetme: String,
        user_num: String,
        channel: String,
        unique_id: String,
        duration: u32,
    },

    /// meetme conference ended
    MeetmeEnd { meetme: String },

    /// meetme user muted/unmuted
    MeetmeMute {
        meetme: String,
        user_num: String,
        channel: String,
        unique_id: String,
        status: String,
    },

    /// meetme user talking
    MeetmeTalking {
        meetme: String,
        user_num: String,
        channel: String,
        unique_id: String,
        status: String,
    },

    /// meetme talk request
    MeetmeTalkRequest {
        meetme: String,
        user_num: String,
        channel: String,
        unique_id: String,
        status: String,
    },

    /// meetme list entry
    MeetmeList {
        meetme: String,
        user_num: String,
        channel: String,
        unique_id: String,
        admin: String,
        muted: String,
        talking: String,
    },

    /// meetme room list entry
    MeetmeListRooms {
        conference: String,
        parties: u32,
        marked: u32,
        locked: String,
    },

    // ── list complete markers ──
    /// device state list complete
    DeviceStateListComplete { items: u32 },

    /// extension state list complete
    ExtensionStateListComplete { items: u32 },

    /// presence state list complete
    PresenceStateListComplete { items: u32 },

    // ── pjsip detail/list ──
    /// aor detail
    AorDetail {
        object_name: String,
        contacts: String,
    },

    /// aor list entry
    AorList { object_name: String },

    /// aor list complete
    AorListComplete { items: u32 },

    /// auth detail
    AuthDetail {
        object_name: String,
        username: String,
    },

    /// auth list entry
    AuthList { object_name: String },

    /// auth list complete
    AuthListComplete { items: u32 },

    /// contact list entry
    ContactList {
        uri: String,
        contact_status: String,
        aor: String,
    },

    /// contact list complete
    ContactListComplete { items: u32 },

    /// contact status detail
    ContactStatusDetail {
        uri: String,
        contact_status: String,
        aor: String,
    },

    /// endpoint detail
    EndpointDetail {
        object_name: String,
        device_state: String,
        active_channels: String,
    },

    /// endpoint detail complete
    EndpointDetailComplete { items: u32 },

    /// endpoint list entry
    EndpointList {
        object_name: String,
        transport: String,
        aor: String,
    },

    /// endpoint list complete
    EndpointListComplete { items: u32 },

    /// identify detail
    IdentifyDetail {
        object_name: String,
        endpoint: String,
    },

    /// transport detail
    TransportDetail {
        object_name: String,
        protocol: String,
    },

    /// resource list detail
    ResourceListDetail { object_name: String },

    /// inbound registration detail
    InboundRegistrationDetail {
        object_name: String,
        contacts: String,
    },

    /// outbound registration detail
    OutboundRegistrationDetail {
        object_name: String,
        server_uri: String,
    },

    /// inbound subscription detail
    InboundSubscriptionDetail { object_name: String },

    /// outbound subscription detail
    OutboundSubscriptionDetail { object_name: String },

    // ── mwi ──
    /// mwi get response
    MWIGet {
        mailbox: String,
        old_messages: u32,
        new_messages: u32,
    },

    /// mwi get complete
    MWIGetComplete { items: u32 },

    // ── misc ──
    /// minivm voicemail notification
    MiniVoiceMail {
        channel: String,
        unique_id: String,
        mailbox: String,
        counter: String,
    },

    /// fax session info
    FAXSession {
        channel: String,
        unique_id: String,
        session_number: String,
    },

    /// fax sessions list entry
    FAXSessionsEntry {
        channel: String,
        session_number: String,
        technology: String,
        state: String,
        files: String,
    },

    /// fax sessions list complete
    FAXSessionsComplete { total: u32 },

    /// fax statistics
    FAXStats {
        current_sessions: u32,
        reserved_sessions: u32,
        transmit_attempts: u32,
        receive_attempts: u32,
        completed_faxes: u32,
        failed_faxes: u32,
    },

    /// do not disturb state changed
    DNDState { channel: String, status: String },

    /// deadlock detected
    DeadlockStart,

    /// malicious call id
    MCID {
        channel: String,
        unique_id: String,
        caller_id_num: String,
        caller_id_name: String,
    },

    /// recognized event whose required field is missing or invalid
    Malformed {
        event_name: String,
        field: String,
        value: Option<String>,
        headers: HashMap<String, String>,
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

        macro_rules! malformed {
            ($field:expr) => {
                return Some(Self::Malformed {
                    event_name: event_name.to_string(),
                    field: $field.to_string(),
                    value: raw
                        .get($field)
                        .map(|value| redact_header_value($field, value)),
                    headers: redacted_headers(raw),
                })
            };
        }
        macro_rules! required_string {
            ($field:expr) => {
                match raw.get($field) {
                    Some(value) => value.to_string(),
                    None => malformed!($field),
                }
            };
        }
        macro_rules! required_parse {
            ($field:expr) => {
                match raw.get($field).and_then(|value| value.parse().ok()) {
                    Some(value) => value,
                    None => malformed!($field),
                }
            };
        }

        let event = match event_name {
            "Newchannel" => Self::NewChannel {
                channel: required_string!("Channel"),
                channel_state: required_string!("ChannelState"),
                channel_state_desc: required_string!("ChannelStateDesc"),
                caller_id_num: required_string!("CallerIDNum"),
                caller_id_name: required_string!("CallerIDName"),
                unique_id: required_string!("Uniqueid"),
                linked_id: required_string!("Linkedid"),
            },
            "Hangup" => Self::Hangup {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                cause: required_parse!("Cause"),
                cause_txt: required_string!("Cause-txt"),
            },
            "Newstate" => Self::Newstate {
                channel: required_string!("Channel"),
                channel_state: required_string!("ChannelState"),
                channel_state_desc: required_string!("ChannelStateDesc"),
                unique_id: required_string!("Uniqueid"),
            },
            "DialBegin" => Self::DialBegin {
                channel: required_string!("Channel"),
                destination: required_string!("DestChannel"),
                dial_string: required_string!("DialString"),
                unique_id: required_string!("Uniqueid"),
                dest_unique_id: required_string!("DestUniqueid"),
            },
            "DialEnd" => Self::DialEnd {
                channel: required_string!("Channel"),
                destination: required_string!("DestChannel"),
                dial_status: required_string!("DialStatus"),
                unique_id: required_string!("Uniqueid"),
                dest_unique_id: required_string!("DestUniqueid"),
            },
            "DTMFBegin" => Self::DtmfBegin {
                channel: required_string!("Channel"),
                digit: required_string!("Digit"),
                direction: required_string!("Direction"),
                unique_id: required_string!("Uniqueid"),
            },
            "DTMFEnd" => Self::DtmfEnd {
                channel: required_string!("Channel"),
                digit: required_string!("Digit"),
                duration_ms: required_parse!("DurationMs"),
                direction: required_string!("Direction"),
                unique_id: required_string!("Uniqueid"),
            },
            "FullyBooted" => Self::FullyBooted {
                status: required_string!("Status"),
            },
            "PeerStatus" => Self::PeerStatus {
                channel_type: required_string!("ChannelType"),
                peer: required_string!("Peer"),
                peer_status: required_string!("PeerStatus"),
            },
            "BridgeCreate" => Self::BridgeCreate {
                bridge_unique_id: required_string!("BridgeUniqueid"),
                bridge_type: required_string!("BridgeType"),
            },
            "BridgeDestroy" => Self::BridgeDestroy {
                bridge_unique_id: required_string!("BridgeUniqueid"),
            },
            "BridgeEnter" => Self::BridgeEnter {
                bridge_unique_id: required_string!("BridgeUniqueid"),
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
            },
            "BridgeLeave" => Self::BridgeLeave {
                bridge_unique_id: required_string!("BridgeUniqueid"),
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
            },

            // core call flow
            "VarSet" => Self::VarSet {
                channel: raw.get("Channel").map(str::to_string),
                variable: required_string!("Variable"),
                value: required_string!("Value"),
                unique_id: raw.get("Uniqueid").map(str::to_string),
            },
            "Hold" => Self::Hold {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                music_class: raw.get("MusicClass").map(|s| s.to_string()),
            },
            "Unhold" => Self::Unhold {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
            },
            "HangupRequest" => Self::HangupRequest {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                cause: required_parse!("Cause"),
            },
            "SoftHangupRequest" => Self::SoftHangupRequest {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                cause: required_parse!("Cause"),
            },
            "NewExten" => Self::NewExten {
                channel: required_string!("Channel"),
                context: required_string!("Context"),
                extension: required_string!("Extension"),
                priority: required_parse!("Priority"),
                application: required_string!("Application"),
                app_data: required_string!("AppData"),
                unique_id: required_string!("Uniqueid"),
            },
            "NewCallerid" => Self::NewCallerid {
                channel: required_string!("Channel"),
                caller_id_num: required_string!("CallerIDNum"),
                caller_id_name: required_string!("CallerIDName"),
                unique_id: required_string!("Uniqueid"),
                cid_calling_pres: required_string!("CID-CallingPres"),
            },
            "NewConnectedLine" => Self::NewConnectedLine {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                connected_line_num: required_string!("ConnectedLineNum"),
                connected_line_name: required_string!("ConnectedLineName"),
            },
            "NewAccountCode" => Self::NewAccountCode {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                account_code: required_string!("AccountCode"),
                old_account_code: required_string!("OldAccountCode"),
            },
            "Rename" => Self::Rename {
                channel: required_string!("Channel"),
                new_name: required_string!("Newname"),
                unique_id: required_string!("Uniqueid"),
            },
            "OriginateResponse" => Self::OriginateResponse {
                action_id: raw.get("ActionID").map(str::to_string),
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                response: required_string!("Response"),
                reason: required_string!("Reason"),
            },
            "DialState" => Self::DialState {
                channel: required_string!("Channel"),
                destination: required_string!("DestChannel"),
                dial_status: required_string!("DialStatus"),
                unique_id: required_string!("Uniqueid"),
                dest_unique_id: required_string!("DestUniqueid"),
            },
            "Flash" => Self::Flash {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
            },
            "Wink" => Self::Wink {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
            },
            "UserEvent" => Self::UserEvent {
                channel: raw.get("Channel").map(|s| s.to_string()),
                unique_id: raw.get("Uniqueid").map(|s| s.to_string()),
                user_event: required_string!("UserEvent"),
                headers: raw.to_map(),
            },

            // transfer
            "AttendedTransfer" => Self::AttendedTransfer {
                result: required_string!("Result"),
                transferer_channel: required_string!("TransfererChannel"),
                transferer_unique_id: required_string!("TransfererUniqueid"),
                transferee_channel: required_string!("TransfereeChannel"),
                transferee_unique_id: required_string!("TransfereeUniqueid"),
            },
            "BlindTransfer" => Self::BlindTransfer {
                result: required_string!("Result"),
                transferer_channel: required_string!("TransfererChannel"),
                transferer_unique_id: required_string!("TransfererUniqueid"),
                extension: required_string!("Extension"),
                context: required_string!("Context"),
            },

            // bridge extended
            "BridgeMerge" => Self::BridgeMerge {
                bridge_unique_id: required_string!("BridgeUniqueid"),
                bridge_type: required_string!("BridgeType"),
                to_bridge_unique_id: required_string!("ToBridgeUniqueid"),
            },
            "BridgeInfoChannel" => Self::BridgeInfoChannel {
                bridge_unique_id: required_string!("BridgeUniqueid"),
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
            },
            "BridgeInfoComplete" => Self::BridgeInfoComplete {
                bridge_unique_id: required_string!("BridgeUniqueid"),
            },
            "BridgeVideoSourceUpdate" => Self::BridgeVideoSourceUpdate {
                bridge_unique_id: required_string!("BridgeUniqueid"),
                bridge_video_source_unique_id: required_string!("BridgeVideoSourceUniqueid"),
            },

            // local channel
            "LocalBridge" => Self::LocalBridge {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                context: required_string!("Context"),
                exten: required_string!("Exten"),
            },
            "LocalOptimizationBegin" => Self::LocalOptimizationBegin {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                source_unique_id: required_string!("SourceUniqueid"),
                dest_unique_id: required_string!("DestUniqueid"),
            },
            "LocalOptimizationEnd" => Self::LocalOptimizationEnd {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
            },

            // cdr / cel
            "Cdr" => Self::Cdr {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                destination: required_string!("Destination"),
                disposition: required_string!("Disposition"),
                duration: required_parse!("Duration"),
                billable_seconds: required_parse!("BillableSeconds"),
                account_code: required_string!("AccountCode"),
                source: required_string!("Source"),
                destination_context: required_string!("DestinationContext"),
            },
            "CEL" => Self::Cel {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                event_name_cel: required_string!("EventName"),
                account_code: required_string!("AccountCode"),
                application_name: required_string!("ApplicationName"),
                application_data: required_string!("ApplicationData"),
            },

            // queue
            "QueueCallerAbandon" => Self::QueueCallerAbandon {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                queue: required_string!("Queue"),
                position: required_parse!("Position"),
                original_position: required_parse!("OriginalPosition"),
                hold_time: required_parse!("HoldTime"),
            },
            "QueueCallerJoin" => Self::QueueCallerJoin {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                queue: required_string!("Queue"),
                position: required_parse!("Position"),
                count: required_parse!("Count"),
            },
            "QueueCallerLeave" => Self::QueueCallerLeave {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                queue: required_string!("Queue"),
                position: required_parse!("Position"),
                count: required_parse!("Count"),
            },
            "QueueMemberAdded" => Self::QueueMemberAdded {
                queue: required_string!("Queue"),
                member_name: required_string!("MemberName"),
                interface: required_string!("Interface"),
                state_interface: required_string!("StateInterface"),
                membership: required_string!("Membership"),
                penalty: required_parse!("Penalty"),
                paused: required_string!("Paused"),
            },
            "QueueMemberRemoved" => Self::QueueMemberRemoved {
                queue: required_string!("Queue"),
                member_name: required_string!("MemberName"),
                interface: required_string!("Interface"),
            },
            "QueueMemberPause" => Self::QueueMemberPause {
                queue: required_string!("Queue"),
                member_name: required_string!("MemberName"),
                interface: required_string!("Interface"),
                paused: required_string!("Paused"),
                reason: required_string!("Reason"),
            },
            "QueueMemberStatus" => Self::QueueMemberStatus {
                queue: required_string!("Queue"),
                member_name: required_string!("MemberName"),
                interface: required_string!("Interface"),
                status: required_parse!("Status"),
                paused: required_string!("Paused"),
                calls_taken: required_parse!("CallsTaken"),
            },
            "QueueMemberPenalty" => Self::QueueMemberPenalty {
                queue: required_string!("Queue"),
                member_name: required_string!("MemberName"),
                interface: required_string!("Interface"),
                penalty: required_parse!("Penalty"),
            },
            "QueueMemberRinginuse" => Self::QueueMemberRinginuse {
                queue: required_string!("Queue"),
                member_name: required_string!("MemberName"),
                interface: required_string!("Interface"),
                ringinuse: required_string!("Ringinuse"),
            },
            "QueueParams" => Self::QueueParams {
                queue: required_string!("Queue"),
                max: required_parse!("Max"),
                strategy: required_string!("Strategy"),
                calls: required_parse!("Calls"),
                holdtime: required_parse!("Holdtime"),
                talktime: required_parse!("Talktime"),
                completed: required_parse!("Completed"),
                abandoned: required_parse!("Abandoned"),
            },
            "QueueEntry" => Self::QueueEntry {
                queue: required_string!("Queue"),
                position: required_parse!("Position"),
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                caller_id_num: required_string!("CallerIDNum"),
                caller_id_name: required_string!("CallerIDName"),
                wait: required_parse!("Wait"),
            },

            // agent
            "AgentCalled" => Self::AgentCalled {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                queue: required_string!("Queue"),
                agent: required_string!("Agent"),
                destination_channel: required_string!("DestinationChannel"),
            },
            "AgentConnect" => Self::AgentConnect {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                queue: required_string!("Queue"),
                agent: required_string!("Agent"),
                hold_time: required_parse!("HoldTime"),
                bridge_unique_id: required_string!("BridgeUniqueid"),
            },
            "AgentComplete" => Self::AgentComplete {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                queue: required_string!("Queue"),
                agent: required_string!("Agent"),
                hold_time: required_parse!("HoldTime"),
                talk_time: required_parse!("TalkTime"),
                reason: required_string!("Reason"),
            },
            "AgentDump" => Self::AgentDump {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                queue: required_string!("Queue"),
                agent: required_string!("Agent"),
            },
            "AgentLogin" => Self::AgentLogin {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                agent: required_string!("Agent"),
            },
            "AgentLogoff" => Self::AgentLogoff {
                agent: required_string!("Agent"),
                logintime: required_parse!("Logintime"),
            },
            "AgentRingNoAnswer" => Self::AgentRingNoAnswer {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                queue: required_string!("Queue"),
                agent: required_string!("Agent"),
                ring_time: required_parse!("RingTime"),
            },
            "Agents" => Self::Agents {
                agent: required_string!("Agent"),
                name: required_string!("Name"),
                status: required_string!("Status"),
                channel: raw.get("Channel").map(|s| s.to_string()),
            },
            "AgentsComplete" => Self::AgentsComplete,

            // conference
            "ConfbridgeStart" => Self::ConfbridgeStart {
                bridge_unique_id: required_string!("BridgeUniqueid"),
                conference: required_string!("Conference"),
            },
            "ConfbridgeEnd" => Self::ConfbridgeEnd {
                bridge_unique_id: required_string!("BridgeUniqueid"),
                conference: required_string!("Conference"),
            },
            "ConfbridgeJoin" => Self::ConfbridgeJoin {
                bridge_unique_id: required_string!("BridgeUniqueid"),
                conference: required_string!("Conference"),
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                admin: required_string!("Admin"),
            },
            "ConfbridgeLeave" => Self::ConfbridgeLeave {
                bridge_unique_id: required_string!("BridgeUniqueid"),
                conference: required_string!("Conference"),
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
            },
            "ConfbridgeList" => Self::ConfbridgeList {
                bridge_unique_id: required_string!("BridgeUniqueid"),
                conference: required_string!("Conference"),
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                admin: required_string!("Admin"),
                muted: required_string!("Muted"),
            },
            "ConfbridgeMute" => Self::ConfbridgeMute {
                bridge_unique_id: required_string!("BridgeUniqueid"),
                conference: required_string!("Conference"),
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
            },
            "ConfbridgeUnmute" => Self::ConfbridgeUnmute {
                bridge_unique_id: required_string!("BridgeUniqueid"),
                conference: required_string!("Conference"),
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
            },
            "ConfbridgeTalking" => Self::ConfbridgeTalking {
                bridge_unique_id: required_string!("BridgeUniqueid"),
                conference: required_string!("Conference"),
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                talking_status: required_string!("TalkingStatus"),
            },
            "ConfbridgeRecord" => Self::ConfbridgeRecord {
                bridge_unique_id: required_string!("BridgeUniqueid"),
                conference: required_string!("Conference"),
            },
            "ConfbridgeStopRecord" => Self::ConfbridgeStopRecord {
                bridge_unique_id: required_string!("BridgeUniqueid"),
                conference: required_string!("Conference"),
            },
            "ConfbridgeListRooms" => Self::ConfbridgeListRooms {
                conference: required_string!("Conference"),
                parties: required_parse!("Parties"),
                marked: required_parse!("Marked"),
                locked: required_string!("Locked"),
            },

            // mixmonitor
            "MixMonitorStart" => Self::MixMonitorStart {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
            },
            "MixMonitorStop" => Self::MixMonitorStop {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
            },
            "MixMonitorMute" => Self::MixMonitorMute {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                direction: required_string!("Direction"),
                state: required_string!("State"),
            },

            // music on hold
            "MusicOnHoldStart" => Self::MusicOnHoldStart {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                class: required_string!("Class"),
            },
            "MusicOnHoldStop" => Self::MusicOnHoldStop {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
            },

            // parking
            "ParkedCall" => Self::ParkedCall {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                parking_lot: required_string!("ParkingLot"),
                parking_space: required_parse!("ParkingSpace"),
                parker_dial_string: required_string!("ParkerDialString"),
                timeout: required_parse!("Timeout"),
            },
            "ParkedCallGiveUp" => Self::ParkedCallGiveUp {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                parking_lot: required_string!("ParkingLot"),
                parking_space: required_parse!("ParkingSpace"),
            },
            "ParkedCallTimeOut" => Self::ParkedCallTimeOut {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                parking_lot: required_string!("ParkingLot"),
                parking_space: required_parse!("ParkingSpace"),
            },
            "ParkedCallSwap" => Self::ParkedCallSwap {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                parking_lot: required_string!("ParkingLot"),
                parking_space: required_parse!("ParkingSpace"),
                parker_channel: required_string!("ParkerChannel"),
            },
            "UnParkedCall" => Self::UnParkedCall {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                parking_lot: required_string!("ParkingLot"),
                parking_space: required_parse!("ParkingSpace"),
                retriever_channel: required_string!("RetrieverChannel"),
            },

            // pickup / spy
            "Pickup" => Self::Pickup {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                target_channel: required_string!("TargetChannel"),
                target_unique_id: required_string!("TargetUniqueid"),
            },
            "ChanSpyStart" => Self::ChanSpyStart {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                spy_channel: required_string!("SpyeeChannel"),
                spy_unique_id: required_string!("SpyeeUniqueid"),
            },
            "ChanSpyStop" => Self::ChanSpyStop {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                spy_channel: required_string!("SpyeeChannel"),
                spy_unique_id: required_string!("SpyeeUniqueid"),
            },

            // channel talking
            "ChannelTalkingStart" => Self::ChannelTalkingStart {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
            },
            "ChannelTalkingStop" => Self::ChannelTalkingStop {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                duration: required_parse!("Duration"),
            },

            // device / presence / extension state
            "DeviceStateChange" => Self::DeviceStateChange {
                device: required_string!("Device"),
                state: required_string!("State"),
            },
            "ExtensionStatus" => Self::ExtensionStatus {
                exten: required_string!("Exten"),
                context: required_string!("Context"),
                hint: required_string!("Hint"),
                status: required_parse!("Status"),
                status_text: required_string!("StatusText"),
            },
            "PresenceStateChange" => Self::PresenceStateChange {
                presentity: required_string!("Presentity"),
                status: required_string!("Status"),
                subtype: required_string!("Subtype"),
                message: required_string!("Message"),
            },
            "PresenceStatus" => Self::PresenceStatus {
                presentity: required_string!("Presentity"),
                status: required_string!("Status"),
                subtype: required_string!("Subtype"),
                message: required_string!("Message"),
            },

            // pjsip / registration
            "ContactStatus" => Self::ContactStatus {
                uri: required_string!("URI"),
                contact_status: required_string!("ContactStatus"),
                aor: required_string!("AOR"),
                endpoint_name: required_string!("EndpointName"),
            },
            "Registry" => Self::Registry {
                channel_type: required_string!("ChannelType"),
                domain: required_string!("Domain"),
                username: required_string!("Username"),
                status: required_string!("Status"),
                cause: required_string!("Cause"),
            },

            // message / voicemail
            "MessageWaiting" => Self::MessageWaiting {
                mailbox: required_string!("Mailbox"),
                waiting: required_string!("Waiting"),
                new_messages: required_parse!("New"),
                old_messages: required_parse!("Old"),
            },
            "VoicemailPasswordChange" => Self::VoicemailPasswordChange {
                context: required_string!("Context"),
                mailbox: required_string!("Mailbox"),
                new_password: required_string!("NewPassword"),
            },

            // rtcp
            "RTCPReceived" => Self::RTCPReceived {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                ssrc: required_string!("SSRC"),
                pt: required_string!("PT"),
                from: required_string!("From"),
            },
            "RTCPSent" => Self::RTCPSent {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                ssrc: required_string!("SSRC"),
                pt: required_string!("PT"),
                to: required_string!("To"),
            },

            // security
            "FailedACL" => Self::FailedACL {
                severity: required_string!("Severity"),
                service: required_string!("Service"),
                account_id: required_string!("AccountID"),
                remote_address: required_string!("RemoteAddress"),
            },
            "InvalidAccountID" => Self::InvalidAccountID {
                severity: required_string!("Severity"),
                service: required_string!("Service"),
                account_id: required_string!("AccountID"),
                remote_address: required_string!("RemoteAddress"),
            },
            "InvalidPassword" => Self::InvalidPassword {
                severity: required_string!("Severity"),
                service: required_string!("Service"),
                account_id: required_string!("AccountID"),
                remote_address: required_string!("RemoteAddress"),
            },
            "ChallengeResponseFailed" => Self::ChallengeResponseFailed {
                severity: required_string!("Severity"),
                service: required_string!("Service"),
                account_id: required_string!("AccountID"),
                remote_address: required_string!("RemoteAddress"),
            },
            "ChallengeSent" => Self::ChallengeSent {
                severity: required_string!("Severity"),
                service: required_string!("Service"),
                account_id: required_string!("AccountID"),
                remote_address: required_string!("RemoteAddress"),
            },
            "SuccessfulAuth" => Self::SuccessfulAuth {
                severity: required_string!("Severity"),
                service: required_string!("Service"),
                account_id: required_string!("AccountID"),
                remote_address: required_string!("RemoteAddress"),
            },
            "SessionLimit" => Self::SessionLimit {
                severity: required_string!("Severity"),
                service: required_string!("Service"),
                account_id: required_string!("AccountID"),
                remote_address: required_string!("RemoteAddress"),
            },
            "UnexpectedAddress" => Self::UnexpectedAddress {
                severity: required_string!("Severity"),
                service: required_string!("Service"),
                account_id: required_string!("AccountID"),
                remote_address: required_string!("RemoteAddress"),
            },
            "RequestBadFormat" => Self::RequestBadFormat {
                severity: required_string!("Severity"),
                service: required_string!("Service"),
                account_id: required_string!("AccountID"),
                remote_address: required_string!("RemoteAddress"),
            },
            "RequestNotAllowed" => Self::RequestNotAllowed {
                severity: required_string!("Severity"),
                service: required_string!("Service"),
                account_id: required_string!("AccountID"),
                remote_address: required_string!("RemoteAddress"),
            },
            "RequestNotSupported" => Self::RequestNotSupported {
                severity: required_string!("Severity"),
                service: required_string!("Service"),
                account_id: required_string!("AccountID"),
                remote_address: required_string!("RemoteAddress"),
            },
            "InvalidTransport" => Self::InvalidTransport {
                severity: required_string!("Severity"),
                service: required_string!("Service"),
                account_id: required_string!("AccountID"),
                remote_address: required_string!("RemoteAddress"),
            },
            "AuthMethodNotAllowed" => Self::AuthMethodNotAllowed {
                severity: required_string!("Severity"),
                service: required_string!("Service"),
                account_id: required_string!("AccountID"),
                remote_address: required_string!("RemoteAddress"),
            },

            // system
            "Shutdown" => Self::Shutdown {
                shutdown_status: required_string!("Shutdown"),
                restart: required_string!("Restart"),
            },
            "Reload" => Self::Reload {
                module: required_string!("Module"),
                status: required_string!("Status"),
            },
            "Load" => Self::Load {
                module: required_string!("Module"),
                status: required_string!("Status"),
            },
            "Unload" => Self::Unload {
                module: required_string!("Module"),
                status: required_string!("Status"),
            },
            "LogChannel" => Self::LogChannel {
                channel_log: required_string!("Channel"),
                enabled: required_string!("Enabled"),
            },
            "LoadAverageLimit" => Self::LoadAverageLimit,
            "MemoryLimit" => Self::MemoryLimit,

            // async agi
            "AsyncAGIStart" => Self::AsyncAGIStart {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                env: required_string!("Env"),
            },
            "AsyncAGIExec" => Self::AsyncAGIExec {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                command_id: required_string!("CommandID"),
                result: required_string!("Result"),
            },
            "AsyncAGIEnd" => Self::AsyncAGIEnd {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
            },
            "AGIExecStart" => Self::AGIExecStart {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                command: required_string!("Command"),
                command_id: required_string!("CommandId"),
            },
            "AGIExecEnd" => Self::AGIExecEnd {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                command: required_string!("Command"),
                command_id: required_string!("CommandId"),
                result_code: required_string!("ResultCode"),
                result: required_string!("Result"),
            },

            // hangup handlers
            "HangupHandlerPush" => Self::HangupHandlerPush {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                handler: required_string!("Handler"),
            },
            "HangupHandlerPop" => Self::HangupHandlerPop {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                handler: required_string!("Handler"),
            },
            "HangupHandlerRun" => Self::HangupHandlerRun {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                handler: required_string!("Handler"),
            },

            // core show / status
            "Status" => Self::Status {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                channel_state: required_string!("ChannelState"),
                caller_id_num: required_string!("CallerIDNum"),
                caller_id_name: required_string!("CallerIDName"),
                account_code: required_string!("AccountCode"),
                context: required_string!("Context"),
                exten: required_string!("Exten"),
                priority: required_parse!("Priority"),
                seconds: required_parse!("Seconds"),
                bridge_id: required_string!("BridgeID"),
                channel_variables: raw.channel_variables.clone(),
            },
            "StatusComplete" => Self::StatusComplete {
                items: required_parse!("Items"),
            },
            "CoreShowChannel" => Self::CoreShowChannel {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                channel_state: required_string!("ChannelState"),
                caller_id_num: required_string!("CallerIDNum"),
                caller_id_name: required_string!("CallerIDName"),
                application: required_string!("Application"),
                application_data: required_string!("ApplicationData"),
                duration: required_string!("Duration"),
                bridge_id: required_string!("BridgeID"),
                channel_variables: raw.channel_variables.clone(),
            },
            "CoreShowChannelsComplete" => Self::CoreShowChannelsComplete {
                listed_channels: required_parse!("ListItems"),
            },
            "CoreShowChannelMapComplete" => Self::CoreShowChannelMapComplete,

            // dahdi
            "DAHDIChannel" => Self::DAHDIChannel {
                dahdi_channel: required_string!("DAHDIChannel"),
                channel: raw.get("Channel").map(|s| s.to_string()),
                unique_id: raw.get("Uniqueid").map(|s| s.to_string()),
            },
            "Alarm" => Self::Alarm {
                alarm: required_string!("Alarm"),
                channel_dahdi: required_string!("Channel"),
            },
            "AlarmClear" => Self::AlarmClear {
                channel_dahdi: required_string!("Channel"),
            },
            "SpanAlarm" => Self::SpanAlarm {
                span: required_parse!("Span"),
                alarm: required_string!("Alarm"),
            },
            "SpanAlarmClear" => Self::SpanAlarmClear {
                span: required_parse!("Span"),
            },

            // aoc
            "AOC-D" => Self::AocD {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                charge_type: required_string!("ChargeType"),
            },
            "AOC-E" => Self::AocE {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                charge_type: required_string!("ChargeType"),
            },
            "AOC-S" => Self::AocS {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
            },

            // fax
            "FAXStatus" => Self::FAXStatus {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                operation: required_string!("Operation"),
                status: required_string!("Status"),
                local_station_id: required_string!("LocalStationID"),
                filename: required_string!("FileName"),
            },
            "ReceiveFAX" => Self::ReceiveFAX {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                local_station_id: required_string!("LocalStationID"),
                remote_station_id: required_string!("RemoteStationID"),
                pages_transferred: required_parse!("PagesTransferred"),
                resolution: required_string!("Resolution"),
                filename: required_string!("FileName"),
            },
            "SendFAX" => Self::SendFAX {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                local_station_id: required_string!("LocalStationID"),
                remote_station_id: required_string!("RemoteStationID"),
                pages_transferred: required_parse!("PagesTransferred"),
                resolution: required_string!("Resolution"),
                filename: required_string!("FileName"),
            },

            // meetme
            "MeetmeJoin" => Self::MeetmeJoin {
                meetme: required_string!("Meetme"),
                user_num: required_string!("Usernum"),
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
            },
            "MeetmeLeave" => Self::MeetmeLeave {
                meetme: required_string!("Meetme"),
                user_num: required_string!("Usernum"),
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                duration: required_parse!("Duration"),
            },
            "MeetmeEnd" => Self::MeetmeEnd {
                meetme: required_string!("Meetme"),
            },
            "MeetmeMute" => Self::MeetmeMute {
                meetme: required_string!("Meetme"),
                user_num: required_string!("Usernum"),
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                status: required_string!("Status"),
            },
            "MeetmeTalking" => Self::MeetmeTalking {
                meetme: required_string!("Meetme"),
                user_num: required_string!("Usernum"),
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                status: required_string!("Status"),
            },
            "MeetmeTalkRequest" => Self::MeetmeTalkRequest {
                meetme: required_string!("Meetme"),
                user_num: required_string!("Usernum"),
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                status: required_string!("Status"),
            },
            "MeetmeList" => Self::MeetmeList {
                meetme: required_string!("Meetme"),
                user_num: required_string!("Usernum"),
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                admin: required_string!("Admin"),
                muted: required_string!("Muted"),
                talking: required_string!("Talking"),
            },
            "MeetmeListRooms" => Self::MeetmeListRooms {
                conference: required_string!("Conference"),
                parties: required_parse!("Parties"),
                marked: required_parse!("Marked"),
                locked: required_string!("Locked"),
            },

            // list complete markers
            "DeviceStateListComplete" => Self::DeviceStateListComplete {
                items: required_parse!("ListItems"),
            },
            "ExtensionStateListComplete" => Self::ExtensionStateListComplete {
                items: required_parse!("ListItems"),
            },
            "PresenceStateListComplete" => Self::PresenceStateListComplete {
                items: required_parse!("ListItems"),
            },

            // pjsip detail/list
            "AorDetail" => Self::AorDetail {
                object_name: required_string!("ObjectName"),
                contacts: required_string!("Contacts"),
            },
            "AorList" => Self::AorList {
                object_name: required_string!("ObjectName"),
            },
            "AorListComplete" => Self::AorListComplete {
                items: required_parse!("ListItems"),
            },
            "AuthDetail" => Self::AuthDetail {
                object_name: required_string!("ObjectName"),
                username: required_string!("Username"),
            },
            "AuthList" => Self::AuthList {
                object_name: required_string!("ObjectName"),
            },
            "AuthListComplete" => Self::AuthListComplete {
                items: required_parse!("ListItems"),
            },
            "ContactList" => Self::ContactList {
                uri: required_string!("URI"),
                contact_status: required_string!("ContactStatus"),
                aor: required_string!("AOR"),
            },
            "ContactListComplete" => Self::ContactListComplete {
                items: required_parse!("ListItems"),
            },
            "ContactStatusDetail" => Self::ContactStatusDetail {
                uri: required_string!("URI"),
                contact_status: required_string!("ContactStatus"),
                aor: required_string!("AOR"),
            },
            "EndpointDetail" => Self::EndpointDetail {
                object_name: required_string!("ObjectName"),
                device_state: required_string!("DeviceState"),
                active_channels: required_string!("ActiveChannels"),
            },
            "EndpointDetailComplete" => Self::EndpointDetailComplete {
                items: required_parse!("ListItems"),
            },
            "EndpointList" => Self::EndpointList {
                object_name: required_string!("ObjectName"),
                transport: required_string!("Transport"),
                aor: required_string!("Aor"),
            },
            "EndpointListComplete" => Self::EndpointListComplete {
                items: required_parse!("ListItems"),
            },
            "IdentifyDetail" => Self::IdentifyDetail {
                object_name: required_string!("ObjectName"),
                endpoint: required_string!("Endpoint"),
            },
            "TransportDetail" => Self::TransportDetail {
                object_name: required_string!("ObjectName"),
                protocol: required_string!("Protocol"),
            },
            "ResourceListDetail" => Self::ResourceListDetail {
                object_name: required_string!("ObjectName"),
            },
            "InboundRegistrationDetail" => Self::InboundRegistrationDetail {
                object_name: required_string!("ObjectName"),
                contacts: required_string!("Contacts"),
            },
            "OutboundRegistrationDetail" => Self::OutboundRegistrationDetail {
                object_name: required_string!("ObjectName"),
                server_uri: required_string!("ServerUri"),
            },
            "InboundSubscriptionDetail" => Self::InboundSubscriptionDetail {
                object_name: required_string!("ObjectName"),
            },
            "OutboundSubscriptionDetail" => Self::OutboundSubscriptionDetail {
                object_name: required_string!("ObjectName"),
            },

            // mwi
            "MWIGet" => Self::MWIGet {
                mailbox: required_string!("Mailbox"),
                old_messages: required_parse!("OldMessages"),
                new_messages: required_parse!("NewMessages"),
            },
            "MWIGetComplete" => Self::MWIGetComplete {
                items: required_parse!("ListItems"),
            },

            // misc
            "MiniVoiceMail" => Self::MiniVoiceMail {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                mailbox: required_string!("Mailbox"),
                counter: required_string!("Counter"),
            },
            "FAXSession" => Self::FAXSession {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                session_number: required_string!("SessionNumber"),
            },
            "FAXSessionsEntry" => Self::FAXSessionsEntry {
                channel: required_string!("Channel"),
                session_number: required_string!("SessionNumber"),
                technology: required_string!("Technology"),
                state: required_string!("State"),
                files: required_string!("Files"),
            },
            "FAXSessionsComplete" => Self::FAXSessionsComplete {
                total: required_parse!("Total"),
            },
            "FAXStats" => Self::FAXStats {
                current_sessions: required_parse!("CurrentSessions"),
                reserved_sessions: required_parse!("ReservedSessions"),
                transmit_attempts: required_parse!("TransmitAttempts"),
                receive_attempts: required_parse!("ReceiveAttempts"),
                completed_faxes: required_parse!("CompletedFAXes"),
                failed_faxes: required_parse!("FailedFAXes"),
            },
            "DNDState" => Self::DNDState {
                channel: required_string!("Channel"),
                status: required_string!("Status"),
            },
            "DeadlockStart" => Self::DeadlockStart,
            "MCID" => Self::MCID {
                channel: required_string!("Channel"),
                unique_id: required_string!("Uniqueid"),
                caller_id_num: required_string!("CallerIDNum"),
                caller_id_name: required_string!("CallerIDName"),
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
            Self::VarSet { .. } => "VarSet",
            Self::Hold { .. } => "Hold",
            Self::Unhold { .. } => "Unhold",
            Self::HangupRequest { .. } => "HangupRequest",
            Self::SoftHangupRequest { .. } => "SoftHangupRequest",
            Self::NewExten { .. } => "NewExten",
            Self::NewCallerid { .. } => "NewCallerid",
            Self::NewConnectedLine { .. } => "NewConnectedLine",
            Self::NewAccountCode { .. } => "NewAccountCode",
            Self::Rename { .. } => "Rename",
            Self::OriginateResponse { .. } => "OriginateResponse",
            Self::DialState { .. } => "DialState",
            Self::Flash { .. } => "Flash",
            Self::Wink { .. } => "Wink",
            Self::UserEvent { .. } => "UserEvent",
            Self::AttendedTransfer { .. } => "AttendedTransfer",
            Self::BlindTransfer { .. } => "BlindTransfer",
            Self::BridgeMerge { .. } => "BridgeMerge",
            Self::BridgeInfoChannel { .. } => "BridgeInfoChannel",
            Self::BridgeInfoComplete { .. } => "BridgeInfoComplete",
            Self::BridgeVideoSourceUpdate { .. } => "BridgeVideoSourceUpdate",
            Self::LocalBridge { .. } => "LocalBridge",
            Self::LocalOptimizationBegin { .. } => "LocalOptimizationBegin",
            Self::LocalOptimizationEnd { .. } => "LocalOptimizationEnd",
            Self::Cdr { .. } => "Cdr",
            Self::Cel { .. } => "CEL",
            Self::QueueCallerAbandon { .. } => "QueueCallerAbandon",
            Self::QueueCallerJoin { .. } => "QueueCallerJoin",
            Self::QueueCallerLeave { .. } => "QueueCallerLeave",
            Self::QueueMemberAdded { .. } => "QueueMemberAdded",
            Self::QueueMemberRemoved { .. } => "QueueMemberRemoved",
            Self::QueueMemberPause { .. } => "QueueMemberPause",
            Self::QueueMemberStatus { .. } => "QueueMemberStatus",
            Self::QueueMemberPenalty { .. } => "QueueMemberPenalty",
            Self::QueueMemberRinginuse { .. } => "QueueMemberRinginuse",
            Self::QueueParams { .. } => "QueueParams",
            Self::QueueEntry { .. } => "QueueEntry",
            Self::AgentCalled { .. } => "AgentCalled",
            Self::AgentConnect { .. } => "AgentConnect",
            Self::AgentComplete { .. } => "AgentComplete",
            Self::AgentDump { .. } => "AgentDump",
            Self::AgentLogin { .. } => "AgentLogin",
            Self::AgentLogoff { .. } => "AgentLogoff",
            Self::AgentRingNoAnswer { .. } => "AgentRingNoAnswer",
            Self::Agents { .. } => "Agents",
            Self::AgentsComplete => "AgentsComplete",
            Self::ConfbridgeStart { .. } => "ConfbridgeStart",
            Self::ConfbridgeEnd { .. } => "ConfbridgeEnd",
            Self::ConfbridgeJoin { .. } => "ConfbridgeJoin",
            Self::ConfbridgeLeave { .. } => "ConfbridgeLeave",
            Self::ConfbridgeList { .. } => "ConfbridgeList",
            Self::ConfbridgeMute { .. } => "ConfbridgeMute",
            Self::ConfbridgeUnmute { .. } => "ConfbridgeUnmute",
            Self::ConfbridgeTalking { .. } => "ConfbridgeTalking",
            Self::ConfbridgeRecord { .. } => "ConfbridgeRecord",
            Self::ConfbridgeStopRecord { .. } => "ConfbridgeStopRecord",
            Self::ConfbridgeListRooms { .. } => "ConfbridgeListRooms",
            Self::MixMonitorStart { .. } => "MixMonitorStart",
            Self::MixMonitorStop { .. } => "MixMonitorStop",
            Self::MixMonitorMute { .. } => "MixMonitorMute",
            Self::MusicOnHoldStart { .. } => "MusicOnHoldStart",
            Self::MusicOnHoldStop { .. } => "MusicOnHoldStop",
            Self::ParkedCall { .. } => "ParkedCall",
            Self::ParkedCallGiveUp { .. } => "ParkedCallGiveUp",
            Self::ParkedCallTimeOut { .. } => "ParkedCallTimeOut",
            Self::ParkedCallSwap { .. } => "ParkedCallSwap",
            Self::UnParkedCall { .. } => "UnParkedCall",
            Self::Pickup { .. } => "Pickup",
            Self::ChanSpyStart { .. } => "ChanSpyStart",
            Self::ChanSpyStop { .. } => "ChanSpyStop",
            Self::ChannelTalkingStart { .. } => "ChannelTalkingStart",
            Self::ChannelTalkingStop { .. } => "ChannelTalkingStop",
            Self::DeviceStateChange { .. } => "DeviceStateChange",
            Self::ExtensionStatus { .. } => "ExtensionStatus",
            Self::PresenceStateChange { .. } => "PresenceStateChange",
            Self::PresenceStatus { .. } => "PresenceStatus",
            Self::ContactStatus { .. } => "ContactStatus",
            Self::Registry { .. } => "Registry",
            Self::MessageWaiting { .. } => "MessageWaiting",
            Self::VoicemailPasswordChange { .. } => "VoicemailPasswordChange",
            Self::RTCPReceived { .. } => "RTCPReceived",
            Self::RTCPSent { .. } => "RTCPSent",
            Self::FailedACL { .. } => "FailedACL",
            Self::InvalidAccountID { .. } => "InvalidAccountID",
            Self::InvalidPassword { .. } => "InvalidPassword",
            Self::ChallengeResponseFailed { .. } => "ChallengeResponseFailed",
            Self::ChallengeSent { .. } => "ChallengeSent",
            Self::SuccessfulAuth { .. } => "SuccessfulAuth",
            Self::SessionLimit { .. } => "SessionLimit",
            Self::UnexpectedAddress { .. } => "UnexpectedAddress",
            Self::RequestBadFormat { .. } => "RequestBadFormat",
            Self::RequestNotAllowed { .. } => "RequestNotAllowed",
            Self::RequestNotSupported { .. } => "RequestNotSupported",
            Self::InvalidTransport { .. } => "InvalidTransport",
            Self::AuthMethodNotAllowed { .. } => "AuthMethodNotAllowed",
            Self::Shutdown { .. } => "Shutdown",
            Self::Reload { .. } => "Reload",
            Self::Load { .. } => "Load",
            Self::Unload { .. } => "Unload",
            Self::LogChannel { .. } => "LogChannel",
            Self::LoadAverageLimit => "LoadAverageLimit",
            Self::MemoryLimit => "MemoryLimit",
            Self::AsyncAGIStart { .. } => "AsyncAGIStart",
            Self::AsyncAGIExec { .. } => "AsyncAGIExec",
            Self::AsyncAGIEnd { .. } => "AsyncAGIEnd",
            Self::AGIExecStart { .. } => "AGIExecStart",
            Self::AGIExecEnd { .. } => "AGIExecEnd",
            Self::HangupHandlerPush { .. } => "HangupHandlerPush",
            Self::HangupHandlerPop { .. } => "HangupHandlerPop",
            Self::HangupHandlerRun { .. } => "HangupHandlerRun",
            Self::Status { .. } => "Status",
            Self::StatusComplete { .. } => "StatusComplete",
            Self::CoreShowChannel { .. } => "CoreShowChannel",
            Self::CoreShowChannelsComplete { .. } => "CoreShowChannelsComplete",
            Self::CoreShowChannelMapComplete => "CoreShowChannelMapComplete",
            Self::DAHDIChannel { .. } => "DAHDIChannel",
            Self::Alarm { .. } => "Alarm",
            Self::AlarmClear { .. } => "AlarmClear",
            Self::SpanAlarm { .. } => "SpanAlarm",
            Self::SpanAlarmClear { .. } => "SpanAlarmClear",
            Self::AocD { .. } => "AOC-D",
            Self::AocE { .. } => "AOC-E",
            Self::AocS { .. } => "AOC-S",
            Self::FAXStatus { .. } => "FAXStatus",
            Self::ReceiveFAX { .. } => "ReceiveFAX",
            Self::SendFAX { .. } => "SendFAX",
            Self::MeetmeJoin { .. } => "MeetmeJoin",
            Self::MeetmeLeave { .. } => "MeetmeLeave",
            Self::MeetmeEnd { .. } => "MeetmeEnd",
            Self::MeetmeMute { .. } => "MeetmeMute",
            Self::MeetmeTalking { .. } => "MeetmeTalking",
            Self::MeetmeTalkRequest { .. } => "MeetmeTalkRequest",
            Self::MeetmeList { .. } => "MeetmeList",
            Self::MeetmeListRooms { .. } => "MeetmeListRooms",
            Self::DeviceStateListComplete { .. } => "DeviceStateListComplete",
            Self::ExtensionStateListComplete { .. } => "ExtensionStateListComplete",
            Self::PresenceStateListComplete { .. } => "PresenceStateListComplete",
            Self::AorDetail { .. } => "AorDetail",
            Self::AorList { .. } => "AorList",
            Self::AorListComplete { .. } => "AorListComplete",
            Self::AuthDetail { .. } => "AuthDetail",
            Self::AuthList { .. } => "AuthList",
            Self::AuthListComplete { .. } => "AuthListComplete",
            Self::ContactList { .. } => "ContactList",
            Self::ContactListComplete { .. } => "ContactListComplete",
            Self::ContactStatusDetail { .. } => "ContactStatusDetail",
            Self::EndpointDetail { .. } => "EndpointDetail",
            Self::EndpointDetailComplete { .. } => "EndpointDetailComplete",
            Self::EndpointList { .. } => "EndpointList",
            Self::EndpointListComplete { .. } => "EndpointListComplete",
            Self::IdentifyDetail { .. } => "IdentifyDetail",
            Self::TransportDetail { .. } => "TransportDetail",
            Self::ResourceListDetail { .. } => "ResourceListDetail",
            Self::InboundRegistrationDetail { .. } => "InboundRegistrationDetail",
            Self::OutboundRegistrationDetail { .. } => "OutboundRegistrationDetail",
            Self::InboundSubscriptionDetail { .. } => "InboundSubscriptionDetail",
            Self::OutboundSubscriptionDetail { .. } => "OutboundSubscriptionDetail",
            Self::MWIGet { .. } => "MWIGet",
            Self::MWIGetComplete { .. } => "MWIGetComplete",
            Self::MiniVoiceMail { .. } => "MiniVoiceMail",
            Self::FAXSession { .. } => "FAXSession",
            Self::FAXSessionsEntry { .. } => "FAXSessionsEntry",
            Self::FAXSessionsComplete { .. } => "FAXSessionsComplete",
            Self::FAXStats { .. } => "FAXStats",
            Self::DNDState { .. } => "DNDState",
            Self::DeadlockStart => "DeadlockStart",
            Self::MCID { .. } => "MCID",
            Self::Malformed { event_name, .. } => event_name,
            Self::Unknown { event_name, .. } => event_name,
        }
    }

    /// whether this event is an event-list completion marker
    ///
    /// Asterisk terminates event-list responses with a `*Complete` event
    /// that carries an `EventList: Complete` header. For typed variants we
    /// match explicitly; for `Unknown` we check the header to avoid false
    /// positives from user events whose names end in "Complete".
    pub fn is_event_list_complete(&self) -> bool {
        match self {
            Self::StatusComplete { .. }
            | Self::CoreShowChannelsComplete { .. }
            | Self::CoreShowChannelMapComplete
            | Self::AgentsComplete
            | Self::BridgeInfoComplete { .. }
            | Self::DeviceStateListComplete { .. }
            | Self::ExtensionStateListComplete { .. }
            | Self::PresenceStateListComplete { .. }
            | Self::AorListComplete { .. }
            | Self::AuthListComplete { .. }
            | Self::ContactListComplete { .. }
            | Self::EndpointDetailComplete { .. }
            | Self::EndpointListComplete { .. }
            | Self::MWIGetComplete { .. }
            | Self::FAXSessionsComplete { .. } => true,
            Self::Malformed { headers, .. } | Self::Unknown { headers, .. } => {
                headers.iter().any(|(key, value)| {
                    key.eq_ignore_ascii_case("EventList") && value.eq_ignore_ascii_case("complete")
                })
            }
            _ => false,
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
            | Self::BridgeLeave { channel, .. }
            | Self::Hold { channel, .. }
            | Self::Unhold { channel, .. }
            | Self::HangupRequest { channel, .. }
            | Self::SoftHangupRequest { channel, .. }
            | Self::NewExten { channel, .. }
            | Self::NewCallerid { channel, .. }
            | Self::NewConnectedLine { channel, .. }
            | Self::NewAccountCode { channel, .. }
            | Self::Rename { channel, .. }
            | Self::OriginateResponse { channel, .. }
            | Self::DialState { channel, .. }
            | Self::Flash { channel, .. }
            | Self::Wink { channel, .. }
            | Self::BridgeInfoChannel { channel, .. }
            | Self::LocalBridge { channel, .. }
            | Self::LocalOptimizationBegin { channel, .. }
            | Self::LocalOptimizationEnd { channel, .. }
            | Self::Cdr { channel, .. }
            | Self::Cel { channel, .. }
            | Self::QueueCallerAbandon { channel, .. }
            | Self::QueueCallerJoin { channel, .. }
            | Self::QueueCallerLeave { channel, .. }
            | Self::QueueEntry { channel, .. }
            | Self::AgentCalled { channel, .. }
            | Self::AgentConnect { channel, .. }
            | Self::AgentComplete { channel, .. }
            | Self::AgentDump { channel, .. }
            | Self::AgentLogin { channel, .. }
            | Self::AgentRingNoAnswer { channel, .. }
            | Self::ConfbridgeJoin { channel, .. }
            | Self::ConfbridgeLeave { channel, .. }
            | Self::ConfbridgeList { channel, .. }
            | Self::ConfbridgeMute { channel, .. }
            | Self::ConfbridgeUnmute { channel, .. }
            | Self::ConfbridgeTalking { channel, .. }
            | Self::MixMonitorStart { channel, .. }
            | Self::MixMonitorStop { channel, .. }
            | Self::MixMonitorMute { channel, .. }
            | Self::MusicOnHoldStart { channel, .. }
            | Self::MusicOnHoldStop { channel, .. }
            | Self::ParkedCall { channel, .. }
            | Self::ParkedCallGiveUp { channel, .. }
            | Self::ParkedCallTimeOut { channel, .. }
            | Self::ParkedCallSwap { channel, .. }
            | Self::UnParkedCall { channel, .. }
            | Self::Pickup { channel, .. }
            | Self::ChanSpyStart { channel, .. }
            | Self::ChanSpyStop { channel, .. }
            | Self::ChannelTalkingStart { channel, .. }
            | Self::ChannelTalkingStop { channel, .. }
            | Self::RTCPReceived { channel, .. }
            | Self::RTCPSent { channel, .. }
            | Self::AsyncAGIStart { channel, .. }
            | Self::AsyncAGIExec { channel, .. }
            | Self::AsyncAGIEnd { channel, .. }
            | Self::AGIExecStart { channel, .. }
            | Self::AGIExecEnd { channel, .. }
            | Self::HangupHandlerPush { channel, .. }
            | Self::HangupHandlerPop { channel, .. }
            | Self::HangupHandlerRun { channel, .. }
            | Self::Status { channel, .. }
            | Self::CoreShowChannel { channel, .. }
            | Self::AocD { channel, .. }
            | Self::AocE { channel, .. }
            | Self::AocS { channel, .. }
            | Self::FAXStatus { channel, .. }
            | Self::ReceiveFAX { channel, .. }
            | Self::SendFAX { channel, .. }
            | Self::MeetmeJoin { channel, .. }
            | Self::MeetmeLeave { channel, .. }
            | Self::MeetmeMute { channel, .. }
            | Self::MeetmeTalking { channel, .. }
            | Self::MeetmeTalkRequest { channel, .. }
            | Self::MeetmeList { channel, .. }
            | Self::MiniVoiceMail { channel, .. }
            | Self::FAXSession { channel, .. }
            | Self::FAXSessionsEntry { channel, .. }
            | Self::DNDState { channel, .. }
            | Self::MCID { channel, .. } => Some(channel),
            // optional channel fields — extract from inner Option
            Self::VarSet { channel, .. }
            | Self::UserEvent { channel, .. }
            | Self::Agents { channel, .. }
            | Self::DAHDIChannel { channel, .. } => channel.as_deref(),
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
            | Self::BridgeLeave { unique_id, .. }
            | Self::Hold { unique_id, .. }
            | Self::Unhold { unique_id, .. }
            | Self::HangupRequest { unique_id, .. }
            | Self::SoftHangupRequest { unique_id, .. }
            | Self::NewExten { unique_id, .. }
            | Self::NewCallerid { unique_id, .. }
            | Self::NewConnectedLine { unique_id, .. }
            | Self::NewAccountCode { unique_id, .. }
            | Self::Rename { unique_id, .. }
            | Self::OriginateResponse { unique_id, .. }
            | Self::DialState { unique_id, .. }
            | Self::Flash { unique_id, .. }
            | Self::Wink { unique_id, .. }
            | Self::BridgeInfoChannel { unique_id, .. }
            | Self::LocalBridge { unique_id, .. }
            | Self::LocalOptimizationBegin { unique_id, .. }
            | Self::LocalOptimizationEnd { unique_id, .. }
            | Self::Cdr { unique_id, .. }
            | Self::Cel { unique_id, .. }
            | Self::QueueCallerAbandon { unique_id, .. }
            | Self::QueueCallerJoin { unique_id, .. }
            | Self::QueueCallerLeave { unique_id, .. }
            | Self::QueueEntry { unique_id, .. }
            | Self::AgentCalled { unique_id, .. }
            | Self::AgentConnect { unique_id, .. }
            | Self::AgentComplete { unique_id, .. }
            | Self::AgentDump { unique_id, .. }
            | Self::AgentLogin { unique_id, .. }
            | Self::AgentRingNoAnswer { unique_id, .. }
            | Self::ConfbridgeJoin { unique_id, .. }
            | Self::ConfbridgeLeave { unique_id, .. }
            | Self::ConfbridgeList { unique_id, .. }
            | Self::ConfbridgeMute { unique_id, .. }
            | Self::ConfbridgeUnmute { unique_id, .. }
            | Self::ConfbridgeTalking { unique_id, .. }
            | Self::MixMonitorStart { unique_id, .. }
            | Self::MixMonitorStop { unique_id, .. }
            | Self::MixMonitorMute { unique_id, .. }
            | Self::MusicOnHoldStart { unique_id, .. }
            | Self::MusicOnHoldStop { unique_id, .. }
            | Self::ParkedCall { unique_id, .. }
            | Self::ParkedCallGiveUp { unique_id, .. }
            | Self::ParkedCallTimeOut { unique_id, .. }
            | Self::ParkedCallSwap { unique_id, .. }
            | Self::UnParkedCall { unique_id, .. }
            | Self::Pickup { unique_id, .. }
            | Self::ChanSpyStart { unique_id, .. }
            | Self::ChanSpyStop { unique_id, .. }
            | Self::ChannelTalkingStart { unique_id, .. }
            | Self::ChannelTalkingStop { unique_id, .. }
            | Self::RTCPReceived { unique_id, .. }
            | Self::RTCPSent { unique_id, .. }
            | Self::AsyncAGIStart { unique_id, .. }
            | Self::AsyncAGIExec { unique_id, .. }
            | Self::AsyncAGIEnd { unique_id, .. }
            | Self::AGIExecStart { unique_id, .. }
            | Self::AGIExecEnd { unique_id, .. }
            | Self::HangupHandlerPush { unique_id, .. }
            | Self::HangupHandlerPop { unique_id, .. }
            | Self::HangupHandlerRun { unique_id, .. }
            | Self::Status { unique_id, .. }
            | Self::CoreShowChannel { unique_id, .. }
            | Self::AocD { unique_id, .. }
            | Self::AocE { unique_id, .. }
            | Self::AocS { unique_id, .. }
            | Self::FAXStatus { unique_id, .. }
            | Self::ReceiveFAX { unique_id, .. }
            | Self::SendFAX { unique_id, .. }
            | Self::MeetmeJoin { unique_id, .. }
            | Self::MeetmeLeave { unique_id, .. }
            | Self::MeetmeMute { unique_id, .. }
            | Self::MeetmeTalking { unique_id, .. }
            | Self::MeetmeTalkRequest { unique_id, .. }
            | Self::MeetmeList { unique_id, .. }
            | Self::MiniVoiceMail { unique_id, .. }
            | Self::FAXSession { unique_id, .. }
            | Self::MCID { unique_id, .. } => Some(unique_id),
            // optional unique_id fields
            Self::VarSet { unique_id, .. }
            | Self::UserEvent { unique_id, .. }
            | Self::DAHDIChannel { unique_id, .. } => unique_id.as_deref(),
            _ => None,
        }
    }
}

// AmiEvent works with the core EventBus
impl asterisk_rs_core::event::Event for AmiEvent {}

const REDACTED_HEADER_VALUE: &str = "[REDACTED]";

fn redacted_headers(raw: &RawAmiMessage) -> HashMap<String, String> {
    raw.headers
        .iter()
        .map(|(key, value)| (key.clone(), redact_header_value(key, value)))
        .collect()
}

fn redact_header_value(key: &str, value: &str) -> String {
    let normalized: String = key
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    if normalized.contains("password")
        || normalized.contains("passwd")
        || normalized.contains("secret")
        || normalized == "md5cred"
        || normalized.contains("credential")
        || normalized.contains("token")
        || normalized.contains("authorization")
        || normalized.contains("apikey")
        || normalized.contains("privatekey")
        || normalized.contains("accesskey")
        || normalized.contains("cookie")
    {
        REDACTED_HEADER_VALUE.to_owned()
    } else {
        value.to_owned()
    }
}
