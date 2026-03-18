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
        channel: String,
        variable: String,
        value: String,
        unique_id: String,
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
                cause: raw.get("Cause").and_then(|s| s.parse().ok()).unwrap_or(0),
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

            // core call flow
            "VarSet" => Self::VarSet {
                channel: get(raw, "Channel"),
                variable: get(raw, "Variable"),
                value: get(raw, "Value"),
                unique_id: get(raw, "Uniqueid"),
            },
            "Hold" => Self::Hold {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                music_class: raw.get("MusicClass").map(|s| s.to_string()),
            },
            "Unhold" => Self::Unhold {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
            },
            "HangupRequest" => Self::HangupRequest {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                cause: raw.get("Cause").and_then(|s| s.parse().ok()).unwrap_or(0),
            },
            "SoftHangupRequest" => Self::SoftHangupRequest {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                cause: raw.get("Cause").and_then(|s| s.parse().ok()).unwrap_or(0),
            },
            "NewExten" => Self::NewExten {
                channel: get(raw, "Channel"),
                context: get(raw, "Context"),
                extension: get(raw, "Extension"),
                priority: raw
                    .get("Priority")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                application: get(raw, "Application"),
                app_data: get(raw, "AppData"),
                unique_id: get(raw, "Uniqueid"),
            },
            "NewCallerid" => Self::NewCallerid {
                channel: get(raw, "Channel"),
                caller_id_num: get(raw, "CallerIDNum"),
                caller_id_name: get(raw, "CallerIDName"),
                unique_id: get(raw, "Uniqueid"),
                cid_calling_pres: get(raw, "CID-CallingPres"),
            },
            "NewConnectedLine" => Self::NewConnectedLine {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                connected_line_num: get(raw, "ConnectedLineNum"),
                connected_line_name: get(raw, "ConnectedLineName"),
            },
            "NewAccountCode" => Self::NewAccountCode {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                account_code: get(raw, "AccountCode"),
                old_account_code: get(raw, "OldAccountCode"),
            },
            "Rename" => Self::Rename {
                channel: get(raw, "Channel"),
                new_name: get(raw, "Newname"),
                unique_id: get(raw, "Uniqueid"),
            },
            "OriginateResponse" => Self::OriginateResponse {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                response: get(raw, "Response"),
                reason: get(raw, "Reason"),
            },
            "DialState" => Self::DialState {
                channel: get(raw, "Channel"),
                destination: get(raw, "DestChannel"),
                dial_status: get(raw, "DialStatus"),
                unique_id: get(raw, "Uniqueid"),
                dest_unique_id: get(raw, "DestUniqueid"),
            },
            "Flash" => Self::Flash {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
            },
            "Wink" => Self::Wink {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
            },
            "UserEvent" => Self::UserEvent {
                channel: raw.get("Channel").map(|s| s.to_string()),
                unique_id: raw.get("Uniqueid").map(|s| s.to_string()),
                user_event: get(raw, "UserEvent"),
                headers: raw.to_map(),
            },

            // transfer
            "AttendedTransfer" => Self::AttendedTransfer {
                result: get(raw, "Result"),
                transferer_channel: get(raw, "TransfererChannel"),
                transferer_unique_id: get(raw, "TransfererUniqueid"),
                transferee_channel: get(raw, "TransfereeChannel"),
                transferee_unique_id: get(raw, "TransfereeUniqueid"),
            },
            "BlindTransfer" => Self::BlindTransfer {
                result: get(raw, "Result"),
                transferer_channel: get(raw, "TransfererChannel"),
                transferer_unique_id: get(raw, "TransfererUniqueid"),
                extension: get(raw, "Extension"),
                context: get(raw, "Context"),
            },

            // bridge extended
            "BridgeMerge" => Self::BridgeMerge {
                bridge_unique_id: get(raw, "BridgeUniqueid"),
                bridge_type: get(raw, "BridgeType"),
                to_bridge_unique_id: get(raw, "ToBridgeUniqueid"),
            },
            "BridgeInfoChannel" => Self::BridgeInfoChannel {
                bridge_unique_id: get(raw, "BridgeUniqueid"),
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
            },
            "BridgeInfoComplete" => Self::BridgeInfoComplete {
                bridge_unique_id: get(raw, "BridgeUniqueid"),
            },
            "BridgeVideoSourceUpdate" => Self::BridgeVideoSourceUpdate {
                bridge_unique_id: get(raw, "BridgeUniqueid"),
                bridge_video_source_unique_id: get(raw, "BridgeVideoSourceUniqueid"),
            },

            // local channel
            "LocalBridge" => Self::LocalBridge {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                context: get(raw, "Context"),
                exten: get(raw, "Exten"),
            },
            "LocalOptimizationBegin" => Self::LocalOptimizationBegin {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                source_unique_id: get(raw, "SourceUniqueid"),
                dest_unique_id: get(raw, "DestUniqueid"),
            },
            "LocalOptimizationEnd" => Self::LocalOptimizationEnd {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
            },

            // cdr / cel
            "Cdr" => Self::Cdr {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                destination: get(raw, "Destination"),
                disposition: get(raw, "Disposition"),
                duration: raw
                    .get("Duration")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                billable_seconds: raw
                    .get("BillableSeconds")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                account_code: get(raw, "AccountCode"),
                source: get(raw, "Source"),
                destination_context: get(raw, "DestinationContext"),
            },
            "CEL" => Self::Cel {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                event_name_cel: get(raw, "EventName"),
                account_code: get(raw, "AccountCode"),
                application_name: get(raw, "ApplicationName"),
                application_data: get(raw, "ApplicationData"),
            },

            // queue
            "QueueCallerAbandon" => Self::QueueCallerAbandon {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                queue: get(raw, "Queue"),
                position: raw
                    .get("Position")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                original_position: raw
                    .get("OriginalPosition")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                hold_time: raw
                    .get("HoldTime")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },
            "QueueCallerJoin" => Self::QueueCallerJoin {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                queue: get(raw, "Queue"),
                position: raw
                    .get("Position")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                count: raw.get("Count").and_then(|s| s.parse().ok()).unwrap_or(0),
            },
            "QueueCallerLeave" => Self::QueueCallerLeave {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                queue: get(raw, "Queue"),
                position: raw
                    .get("Position")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                count: raw.get("Count").and_then(|s| s.parse().ok()).unwrap_or(0),
            },
            "QueueMemberAdded" => Self::QueueMemberAdded {
                queue: get(raw, "Queue"),
                member_name: get(raw, "MemberName"),
                interface: get(raw, "Interface"),
                state_interface: get(raw, "StateInterface"),
                membership: get(raw, "Membership"),
                penalty: raw.get("Penalty").and_then(|s| s.parse().ok()).unwrap_or(0),
                paused: get(raw, "Paused"),
            },
            "QueueMemberRemoved" => Self::QueueMemberRemoved {
                queue: get(raw, "Queue"),
                member_name: get(raw, "MemberName"),
                interface: get(raw, "Interface"),
            },
            "QueueMemberPause" => Self::QueueMemberPause {
                queue: get(raw, "Queue"),
                member_name: get(raw, "MemberName"),
                interface: get(raw, "Interface"),
                paused: get(raw, "Paused"),
                reason: get(raw, "Reason"),
            },
            "QueueMemberStatus" => Self::QueueMemberStatus {
                queue: get(raw, "Queue"),
                member_name: get(raw, "MemberName"),
                interface: get(raw, "Interface"),
                status: raw.get("Status").and_then(|s| s.parse().ok()).unwrap_or(0),
                paused: get(raw, "Paused"),
                calls_taken: raw
                    .get("CallsTaken")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },
            "QueueMemberPenalty" => Self::QueueMemberPenalty {
                queue: get(raw, "Queue"),
                member_name: get(raw, "MemberName"),
                interface: get(raw, "Interface"),
                penalty: raw.get("Penalty").and_then(|s| s.parse().ok()).unwrap_or(0),
            },
            "QueueMemberRinginuse" => Self::QueueMemberRinginuse {
                queue: get(raw, "Queue"),
                member_name: get(raw, "MemberName"),
                interface: get(raw, "Interface"),
                ringinuse: get(raw, "Ringinuse"),
            },
            "QueueParams" => Self::QueueParams {
                queue: get(raw, "Queue"),
                max: raw.get("Max").and_then(|s| s.parse().ok()).unwrap_or(0),
                strategy: get(raw, "Strategy"),
                calls: raw.get("Calls").and_then(|s| s.parse().ok()).unwrap_or(0),
                holdtime: raw
                    .get("Holdtime")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                talktime: raw
                    .get("Talktime")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                completed: raw
                    .get("Completed")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                abandoned: raw
                    .get("Abandoned")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },
            "QueueEntry" => Self::QueueEntry {
                queue: get(raw, "Queue"),
                position: raw
                    .get("Position")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                caller_id_num: get(raw, "CallerIDNum"),
                caller_id_name: get(raw, "CallerIDName"),
                wait: raw.get("Wait").and_then(|s| s.parse().ok()).unwrap_or(0),
            },

            // agent
            "AgentCalled" => Self::AgentCalled {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                queue: get(raw, "Queue"),
                agent: get(raw, "Agent"),
                destination_channel: get(raw, "DestinationChannel"),
            },
            "AgentConnect" => Self::AgentConnect {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                queue: get(raw, "Queue"),
                agent: get(raw, "Agent"),
                hold_time: raw
                    .get("HoldTime")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                bridge_unique_id: get(raw, "BridgeUniqueid"),
            },
            "AgentComplete" => Self::AgentComplete {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                queue: get(raw, "Queue"),
                agent: get(raw, "Agent"),
                hold_time: raw
                    .get("HoldTime")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                talk_time: raw
                    .get("TalkTime")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                reason: get(raw, "Reason"),
            },
            "AgentDump" => Self::AgentDump {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                queue: get(raw, "Queue"),
                agent: get(raw, "Agent"),
            },
            "AgentLogin" => Self::AgentLogin {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                agent: get(raw, "Agent"),
            },
            "AgentLogoff" => Self::AgentLogoff {
                agent: get(raw, "Agent"),
                logintime: raw
                    .get("Logintime")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },
            "AgentRingNoAnswer" => Self::AgentRingNoAnswer {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                queue: get(raw, "Queue"),
                agent: get(raw, "Agent"),
                ring_time: raw
                    .get("RingTime")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },
            "Agents" => Self::Agents {
                agent: get(raw, "Agent"),
                name: get(raw, "Name"),
                status: get(raw, "Status"),
                channel: raw.get("Channel").map(|s| s.to_string()),
            },
            "AgentsComplete" => Self::AgentsComplete,

            // conference
            "ConfbridgeStart" => Self::ConfbridgeStart {
                bridge_unique_id: get(raw, "BridgeUniqueid"),
                conference: get(raw, "Conference"),
            },
            "ConfbridgeEnd" => Self::ConfbridgeEnd {
                bridge_unique_id: get(raw, "BridgeUniqueid"),
                conference: get(raw, "Conference"),
            },
            "ConfbridgeJoin" => Self::ConfbridgeJoin {
                bridge_unique_id: get(raw, "BridgeUniqueid"),
                conference: get(raw, "Conference"),
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                admin: get(raw, "Admin"),
            },
            "ConfbridgeLeave" => Self::ConfbridgeLeave {
                bridge_unique_id: get(raw, "BridgeUniqueid"),
                conference: get(raw, "Conference"),
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
            },
            "ConfbridgeList" => Self::ConfbridgeList {
                bridge_unique_id: get(raw, "BridgeUniqueid"),
                conference: get(raw, "Conference"),
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                admin: get(raw, "Admin"),
                muted: get(raw, "Muted"),
            },
            "ConfbridgeMute" => Self::ConfbridgeMute {
                bridge_unique_id: get(raw, "BridgeUniqueid"),
                conference: get(raw, "Conference"),
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
            },
            "ConfbridgeUnmute" => Self::ConfbridgeUnmute {
                bridge_unique_id: get(raw, "BridgeUniqueid"),
                conference: get(raw, "Conference"),
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
            },
            "ConfbridgeTalking" => Self::ConfbridgeTalking {
                bridge_unique_id: get(raw, "BridgeUniqueid"),
                conference: get(raw, "Conference"),
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                talking_status: get(raw, "TalkingStatus"),
            },
            "ConfbridgeRecord" => Self::ConfbridgeRecord {
                bridge_unique_id: get(raw, "BridgeUniqueid"),
                conference: get(raw, "Conference"),
            },
            "ConfbridgeStopRecord" => Self::ConfbridgeStopRecord {
                bridge_unique_id: get(raw, "BridgeUniqueid"),
                conference: get(raw, "Conference"),
            },
            "ConfbridgeListRooms" => Self::ConfbridgeListRooms {
                conference: get(raw, "Conference"),
                parties: raw.get("Parties").and_then(|s| s.parse().ok()).unwrap_or(0),
                marked: raw.get("Marked").and_then(|s| s.parse().ok()).unwrap_or(0),
                locked: get(raw, "Locked"),
            },

            // mixmonitor
            "MixMonitorStart" => Self::MixMonitorStart {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
            },
            "MixMonitorStop" => Self::MixMonitorStop {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
            },
            "MixMonitorMute" => Self::MixMonitorMute {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                direction: get(raw, "Direction"),
                state: get(raw, "State"),
            },

            // music on hold
            "MusicOnHoldStart" => Self::MusicOnHoldStart {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                class: get(raw, "Class"),
            },
            "MusicOnHoldStop" => Self::MusicOnHoldStop {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
            },

            // parking
            "ParkedCall" => Self::ParkedCall {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                parking_lot: get(raw, "ParkingLot"),
                parking_space: raw
                    .get("ParkingSpace")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                parker_dial_string: get(raw, "ParkerDialString"),
                timeout: raw.get("Timeout").and_then(|s| s.parse().ok()).unwrap_or(0),
            },
            "ParkedCallGiveUp" => Self::ParkedCallGiveUp {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                parking_lot: get(raw, "ParkingLot"),
                parking_space: raw
                    .get("ParkingSpace")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },
            "ParkedCallTimeOut" => Self::ParkedCallTimeOut {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                parking_lot: get(raw, "ParkingLot"),
                parking_space: raw
                    .get("ParkingSpace")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },
            "ParkedCallSwap" => Self::ParkedCallSwap {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                parking_lot: get(raw, "ParkingLot"),
                parking_space: raw
                    .get("ParkingSpace")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                parker_channel: get(raw, "ParkerChannel"),
            },
            "UnParkedCall" => Self::UnParkedCall {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                parking_lot: get(raw, "ParkingLot"),
                parking_space: raw
                    .get("ParkingSpace")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                retriever_channel: get(raw, "RetrieverChannel"),
            },

            // pickup / spy
            "Pickup" => Self::Pickup {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                target_channel: get(raw, "TargetChannel"),
                target_unique_id: get(raw, "TargetUniqueid"),
            },
            "ChanSpyStart" => Self::ChanSpyStart {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                spy_channel: get(raw, "SpyeeChannel"),
                spy_unique_id: get(raw, "SpyeeUniqueid"),
            },
            "ChanSpyStop" => Self::ChanSpyStop {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                spy_channel: get(raw, "SpyeeChannel"),
                spy_unique_id: get(raw, "SpyeeUniqueid"),
            },

            // channel talking
            "ChannelTalkingStart" => Self::ChannelTalkingStart {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
            },
            "ChannelTalkingStop" => Self::ChannelTalkingStop {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                duration: raw
                    .get("Duration")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },

            // device / presence / extension state
            "DeviceStateChange" => Self::DeviceStateChange {
                device: get(raw, "Device"),
                state: get(raw, "State"),
            },
            "ExtensionStatus" => Self::ExtensionStatus {
                exten: get(raw, "Exten"),
                context: get(raw, "Context"),
                hint: get(raw, "Hint"),
                status: raw.get("Status").and_then(|s| s.parse().ok()).unwrap_or(0),
                status_text: get(raw, "StatusText"),
            },
            "PresenceStateChange" => Self::PresenceStateChange {
                presentity: get(raw, "Presentity"),
                status: get(raw, "Status"),
                subtype: get(raw, "Subtype"),
                message: get(raw, "Message"),
            },
            "PresenceStatus" => Self::PresenceStatus {
                presentity: get(raw, "Presentity"),
                status: get(raw, "Status"),
                subtype: get(raw, "Subtype"),
                message: get(raw, "Message"),
            },

            // pjsip / registration
            "ContactStatus" => Self::ContactStatus {
                uri: get(raw, "URI"),
                contact_status: get(raw, "ContactStatus"),
                aor: get(raw, "AOR"),
                endpoint_name: get(raw, "EndpointName"),
            },
            "Registry" => Self::Registry {
                channel_type: get(raw, "ChannelType"),
                domain: get(raw, "Domain"),
                username: get(raw, "Username"),
                status: get(raw, "Status"),
                cause: get(raw, "Cause"),
            },

            // message / voicemail
            "MessageWaiting" => Self::MessageWaiting {
                mailbox: get(raw, "Mailbox"),
                waiting: get(raw, "Waiting"),
                new_messages: raw.get("New").and_then(|s| s.parse().ok()).unwrap_or(0),
                old_messages: raw.get("Old").and_then(|s| s.parse().ok()).unwrap_or(0),
            },
            "VoicemailPasswordChange" => Self::VoicemailPasswordChange {
                context: get(raw, "Context"),
                mailbox: get(raw, "Mailbox"),
                new_password: get(raw, "NewPassword"),
            },

            // rtcp
            "RTCPReceived" => Self::RTCPReceived {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                ssrc: get(raw, "SSRC"),
                pt: get(raw, "PT"),
                from: get(raw, "From"),
            },
            "RTCPSent" => Self::RTCPSent {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                ssrc: get(raw, "SSRC"),
                pt: get(raw, "PT"),
                to: get(raw, "To"),
            },

            // security
            "FailedACL" => Self::FailedACL {
                severity: get(raw, "Severity"),
                service: get(raw, "Service"),
                account_id: get(raw, "AccountID"),
                remote_address: get(raw, "RemoteAddress"),
            },
            "InvalidAccountID" => Self::InvalidAccountID {
                severity: get(raw, "Severity"),
                service: get(raw, "Service"),
                account_id: get(raw, "AccountID"),
                remote_address: get(raw, "RemoteAddress"),
            },
            "InvalidPassword" => Self::InvalidPassword {
                severity: get(raw, "Severity"),
                service: get(raw, "Service"),
                account_id: get(raw, "AccountID"),
                remote_address: get(raw, "RemoteAddress"),
            },
            "ChallengeResponseFailed" => Self::ChallengeResponseFailed {
                severity: get(raw, "Severity"),
                service: get(raw, "Service"),
                account_id: get(raw, "AccountID"),
                remote_address: get(raw, "RemoteAddress"),
            },
            "ChallengeSent" => Self::ChallengeSent {
                severity: get(raw, "Severity"),
                service: get(raw, "Service"),
                account_id: get(raw, "AccountID"),
                remote_address: get(raw, "RemoteAddress"),
            },
            "SuccessfulAuth" => Self::SuccessfulAuth {
                severity: get(raw, "Severity"),
                service: get(raw, "Service"),
                account_id: get(raw, "AccountID"),
                remote_address: get(raw, "RemoteAddress"),
            },
            "SessionLimit" => Self::SessionLimit {
                severity: get(raw, "Severity"),
                service: get(raw, "Service"),
                account_id: get(raw, "AccountID"),
                remote_address: get(raw, "RemoteAddress"),
            },
            "UnexpectedAddress" => Self::UnexpectedAddress {
                severity: get(raw, "Severity"),
                service: get(raw, "Service"),
                account_id: get(raw, "AccountID"),
                remote_address: get(raw, "RemoteAddress"),
            },
            "RequestBadFormat" => Self::RequestBadFormat {
                severity: get(raw, "Severity"),
                service: get(raw, "Service"),
                account_id: get(raw, "AccountID"),
                remote_address: get(raw, "RemoteAddress"),
            },
            "RequestNotAllowed" => Self::RequestNotAllowed {
                severity: get(raw, "Severity"),
                service: get(raw, "Service"),
                account_id: get(raw, "AccountID"),
                remote_address: get(raw, "RemoteAddress"),
            },
            "RequestNotSupported" => Self::RequestNotSupported {
                severity: get(raw, "Severity"),
                service: get(raw, "Service"),
                account_id: get(raw, "AccountID"),
                remote_address: get(raw, "RemoteAddress"),
            },
            "InvalidTransport" => Self::InvalidTransport {
                severity: get(raw, "Severity"),
                service: get(raw, "Service"),
                account_id: get(raw, "AccountID"),
                remote_address: get(raw, "RemoteAddress"),
            },
            "AuthMethodNotAllowed" => Self::AuthMethodNotAllowed {
                severity: get(raw, "Severity"),
                service: get(raw, "Service"),
                account_id: get(raw, "AccountID"),
                remote_address: get(raw, "RemoteAddress"),
            },

            // system
            "Shutdown" => Self::Shutdown {
                shutdown_status: get(raw, "Shutdown"),
                restart: get(raw, "Restart"),
            },
            "Reload" => Self::Reload {
                module: get(raw, "Module"),
                status: get(raw, "Status"),
            },
            "Load" => Self::Load {
                module: get(raw, "Module"),
                status: get(raw, "Status"),
            },
            "Unload" => Self::Unload {
                module: get(raw, "Module"),
                status: get(raw, "Status"),
            },
            "LogChannel" => Self::LogChannel {
                channel_log: get(raw, "Channel"),
                enabled: get(raw, "Enabled"),
            },
            "LoadAverageLimit" => Self::LoadAverageLimit,
            "MemoryLimit" => Self::MemoryLimit,

            // async agi
            "AsyncAGIStart" => Self::AsyncAGIStart {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                env: get(raw, "Env"),
            },
            "AsyncAGIExec" => Self::AsyncAGIExec {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                command_id: get(raw, "CommandID"),
                result: get(raw, "Result"),
            },
            "AsyncAGIEnd" => Self::AsyncAGIEnd {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
            },
            "AGIExecStart" => Self::AGIExecStart {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                command: get(raw, "Command"),
                command_id: get(raw, "CommandId"),
            },
            "AGIExecEnd" => Self::AGIExecEnd {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                command: get(raw, "Command"),
                command_id: get(raw, "CommandId"),
                result_code: get(raw, "ResultCode"),
                result: get(raw, "Result"),
            },

            // hangup handlers
            "HangupHandlerPush" => Self::HangupHandlerPush {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                handler: get(raw, "Handler"),
            },
            "HangupHandlerPop" => Self::HangupHandlerPop {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                handler: get(raw, "Handler"),
            },
            "HangupHandlerRun" => Self::HangupHandlerRun {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                handler: get(raw, "Handler"),
            },

            // core show / status
            "Status" => Self::Status {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                channel_state: get(raw, "ChannelState"),
                caller_id_num: get(raw, "CallerIDNum"),
                caller_id_name: get(raw, "CallerIDName"),
                account_code: get(raw, "AccountCode"),
                context: get(raw, "Context"),
                exten: get(raw, "Exten"),
                priority: raw
                    .get("Priority")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                seconds: raw.get("Seconds").and_then(|s| s.parse().ok()).unwrap_or(0),
                bridge_id: get(raw, "BridgeID"),
            },
            "StatusComplete" => Self::StatusComplete {
                items: raw.get("Items").and_then(|s| s.parse().ok()).unwrap_or(0),
            },
            "CoreShowChannel" => Self::CoreShowChannel {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                channel_state: get(raw, "ChannelState"),
                caller_id_num: get(raw, "CallerIDNum"),
                caller_id_name: get(raw, "CallerIDName"),
                application: get(raw, "Application"),
                application_data: get(raw, "ApplicationData"),
                duration: get(raw, "Duration"),
                bridge_id: get(raw, "BridgeID"),
            },
            "CoreShowChannelsComplete" => Self::CoreShowChannelsComplete {
                listed_channels: raw
                    .get("ListItems")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },
            "CoreShowChannelMapComplete" => Self::CoreShowChannelMapComplete,

            // dahdi
            "DAHDIChannel" => Self::DAHDIChannel {
                dahdi_channel: get(raw, "DAHDIChannel"),
                channel: raw.get("Channel").map(|s| s.to_string()),
                unique_id: raw.get("Uniqueid").map(|s| s.to_string()),
            },
            "Alarm" => Self::Alarm {
                alarm: get(raw, "Alarm"),
                channel_dahdi: get(raw, "Channel"),
            },
            "AlarmClear" => Self::AlarmClear {
                channel_dahdi: get(raw, "Channel"),
            },
            "SpanAlarm" => Self::SpanAlarm {
                span: raw.get("Span").and_then(|s| s.parse().ok()).unwrap_or(0),
                alarm: get(raw, "Alarm"),
            },
            "SpanAlarmClear" => Self::SpanAlarmClear {
                span: raw.get("Span").and_then(|s| s.parse().ok()).unwrap_or(0),
            },

            // aoc
            "AOC-D" => Self::AocD {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                charge_type: get(raw, "ChargeType"),
            },
            "AOC-E" => Self::AocE {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                charge_type: get(raw, "ChargeType"),
            },
            "AOC-S" => Self::AocS {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
            },

            // fax
            "FAXStatus" => Self::FAXStatus {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                operation: get(raw, "Operation"),
                status: get(raw, "Status"),
                local_station_id: get(raw, "LocalStationID"),
                filename: get(raw, "FileName"),
            },
            "ReceiveFAX" => Self::ReceiveFAX {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                local_station_id: get(raw, "LocalStationID"),
                remote_station_id: get(raw, "RemoteStationID"),
                pages_transferred: raw
                    .get("PagesTransferred")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                resolution: get(raw, "Resolution"),
                filename: get(raw, "FileName"),
            },
            "SendFAX" => Self::SendFAX {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                local_station_id: get(raw, "LocalStationID"),
                remote_station_id: get(raw, "RemoteStationID"),
                pages_transferred: raw
                    .get("PagesTransferred")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                resolution: get(raw, "Resolution"),
                filename: get(raw, "FileName"),
            },

            // meetme
            "MeetmeJoin" => Self::MeetmeJoin {
                meetme: get(raw, "Meetme"),
                user_num: get(raw, "Usernum"),
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
            },
            "MeetmeLeave" => Self::MeetmeLeave {
                meetme: get(raw, "Meetme"),
                user_num: get(raw, "Usernum"),
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                duration: raw
                    .get("Duration")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },
            "MeetmeEnd" => Self::MeetmeEnd {
                meetme: get(raw, "Meetme"),
            },
            "MeetmeMute" => Self::MeetmeMute {
                meetme: get(raw, "Meetme"),
                user_num: get(raw, "Usernum"),
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                status: get(raw, "Status"),
            },
            "MeetmeTalking" => Self::MeetmeTalking {
                meetme: get(raw, "Meetme"),
                user_num: get(raw, "Usernum"),
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                status: get(raw, "Status"),
            },
            "MeetmeTalkRequest" => Self::MeetmeTalkRequest {
                meetme: get(raw, "Meetme"),
                user_num: get(raw, "Usernum"),
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                status: get(raw, "Status"),
            },
            "MeetmeList" => Self::MeetmeList {
                meetme: get(raw, "Meetme"),
                user_num: get(raw, "Usernum"),
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                admin: get(raw, "Admin"),
                muted: get(raw, "Muted"),
                talking: get(raw, "Talking"),
            },
            "MeetmeListRooms" => Self::MeetmeListRooms {
                conference: get(raw, "Conference"),
                parties: raw.get("Parties").and_then(|s| s.parse().ok()).unwrap_or(0),
                marked: raw.get("Marked").and_then(|s| s.parse().ok()).unwrap_or(0),
                locked: get(raw, "Locked"),
            },

            // list complete markers
            "DeviceStateListComplete" => Self::DeviceStateListComplete {
                items: raw
                    .get("ListItems")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },
            "ExtensionStateListComplete" => Self::ExtensionStateListComplete {
                items: raw
                    .get("ListItems")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },
            "PresenceStateListComplete" => Self::PresenceStateListComplete {
                items: raw
                    .get("ListItems")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },

            // pjsip detail/list
            "AorDetail" => Self::AorDetail {
                object_name: get(raw, "ObjectName"),
                contacts: get(raw, "Contacts"),
            },
            "AorList" => Self::AorList {
                object_name: get(raw, "ObjectName"),
            },
            "AorListComplete" => Self::AorListComplete {
                items: raw
                    .get("ListItems")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },
            "AuthDetail" => Self::AuthDetail {
                object_name: get(raw, "ObjectName"),
                username: get(raw, "Username"),
            },
            "AuthList" => Self::AuthList {
                object_name: get(raw, "ObjectName"),
            },
            "AuthListComplete" => Self::AuthListComplete {
                items: raw
                    .get("ListItems")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },
            "ContactList" => Self::ContactList {
                uri: get(raw, "URI"),
                contact_status: get(raw, "ContactStatus"),
                aor: get(raw, "AOR"),
            },
            "ContactListComplete" => Self::ContactListComplete {
                items: raw
                    .get("ListItems")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },
            "ContactStatusDetail" => Self::ContactStatusDetail {
                uri: get(raw, "URI"),
                contact_status: get(raw, "ContactStatus"),
                aor: get(raw, "AOR"),
            },
            "EndpointDetail" => Self::EndpointDetail {
                object_name: get(raw, "ObjectName"),
                device_state: get(raw, "DeviceState"),
                active_channels: get(raw, "ActiveChannels"),
            },
            "EndpointDetailComplete" => Self::EndpointDetailComplete {
                items: raw
                    .get("ListItems")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },
            "EndpointList" => Self::EndpointList {
                object_name: get(raw, "ObjectName"),
                transport: get(raw, "Transport"),
                aor: get(raw, "Aor"),
            },
            "EndpointListComplete" => Self::EndpointListComplete {
                items: raw
                    .get("ListItems")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },
            "IdentifyDetail" => Self::IdentifyDetail {
                object_name: get(raw, "ObjectName"),
                endpoint: get(raw, "Endpoint"),
            },
            "TransportDetail" => Self::TransportDetail {
                object_name: get(raw, "ObjectName"),
                protocol: get(raw, "Protocol"),
            },
            "ResourceListDetail" => Self::ResourceListDetail {
                object_name: get(raw, "ObjectName"),
            },
            "InboundRegistrationDetail" => Self::InboundRegistrationDetail {
                object_name: get(raw, "ObjectName"),
                contacts: get(raw, "Contacts"),
            },
            "OutboundRegistrationDetail" => Self::OutboundRegistrationDetail {
                object_name: get(raw, "ObjectName"),
                server_uri: get(raw, "ServerUri"),
            },
            "InboundSubscriptionDetail" => Self::InboundSubscriptionDetail {
                object_name: get(raw, "ObjectName"),
            },
            "OutboundSubscriptionDetail" => Self::OutboundSubscriptionDetail {
                object_name: get(raw, "ObjectName"),
            },

            // mwi
            "MWIGet" => Self::MWIGet {
                mailbox: get(raw, "Mailbox"),
                old_messages: raw
                    .get("OldMessages")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                new_messages: raw
                    .get("NewMessages")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },
            "MWIGetComplete" => Self::MWIGetComplete {
                items: raw
                    .get("ListItems")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },

            // misc
            "MiniVoiceMail" => Self::MiniVoiceMail {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                mailbox: get(raw, "Mailbox"),
                counter: get(raw, "Counter"),
            },
            "FAXSession" => Self::FAXSession {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                session_number: get(raw, "SessionNumber"),
            },
            "FAXSessionsEntry" => Self::FAXSessionsEntry {
                channel: get(raw, "Channel"),
                session_number: get(raw, "SessionNumber"),
                technology: get(raw, "Technology"),
                state: get(raw, "State"),
                files: get(raw, "Files"),
            },
            "FAXSessionsComplete" => Self::FAXSessionsComplete {
                total: raw.get("Total").and_then(|s| s.parse().ok()).unwrap_or(0),
            },
            "FAXStats" => Self::FAXStats {
                current_sessions: raw
                    .get("CurrentSessions")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                reserved_sessions: raw
                    .get("ReservedSessions")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                transmit_attempts: raw
                    .get("TransmitAttempts")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                receive_attempts: raw
                    .get("ReceiveAttempts")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                completed_faxes: raw
                    .get("CompletedFAXes")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                failed_faxes: raw
                    .get("FailedFAXes")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            },
            "DNDState" => Self::DNDState {
                channel: get(raw, "Channel"),
                status: get(raw, "Status"),
            },
            "DeadlockStart" => Self::DeadlockStart,
            "MCID" => Self::MCID {
                channel: get(raw, "Channel"),
                unique_id: get(raw, "Uniqueid"),
                caller_id_num: get(raw, "CallerIDNum"),
                caller_id_name: get(raw, "CallerIDName"),
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
            | Self::BridgeLeave { channel, .. }
            | Self::VarSet { channel, .. }
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
            Self::UserEvent { channel, .. }
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
            | Self::VarSet { unique_id, .. }
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
            Self::UserEvent { unique_id, .. } | Self::DAHDIChannel { unique_id, .. } => {
                unique_id.as_deref()
            }
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

    #[test]
    fn parse_varset_event() {
        let raw = RawAmiMessage {
            headers: vec![
                ("Event".into(), "VarSet".into()),
                ("Channel".into(), "PJSIP/100-0001".into()),
                ("Variable".into(), "DIALSTATUS".into()),
                ("Value".into(), "ANSWER".into()),
                ("Uniqueid".into(), "1234.5".into()),
            ],
        };
        let event = AmiEvent::from_raw(&raw).expect("should parse");
        assert_eq!(event.event_name(), "VarSet");
        assert_eq!(event.channel(), Some("PJSIP/100-0001"));
        if let AmiEvent::VarSet {
            variable, value, ..
        } = &event
        {
            assert_eq!(variable, "DIALSTATUS");
            assert_eq!(value, "ANSWER");
        } else {
            panic!("expected VarSet");
        }
    }

    #[test]
    fn parse_hold_event() {
        let raw = RawAmiMessage {
            headers: vec![
                ("Event".into(), "Hold".into()),
                ("Channel".into(), "PJSIP/200-0002".into()),
                ("Uniqueid".into(), "5678.1".into()),
                ("MusicClass".into(), "default".into()),
            ],
        };
        let event = AmiEvent::from_raw(&raw).expect("should parse");
        assert_eq!(event.event_name(), "Hold");
        if let AmiEvent::Hold { music_class, .. } = &event {
            assert_eq!(music_class.as_deref(), Some("default"));
        } else {
            panic!("expected Hold");
        }
    }

    #[test]
    fn parse_originate_response_event() {
        let raw = RawAmiMessage {
            headers: vec![
                ("Event".into(), "OriginateResponse".into()),
                ("Channel".into(), "PJSIP/100-0001".into()),
                ("Uniqueid".into(), "9999.1".into()),
                ("Response".into(), "Success".into()),
                ("Reason".into(), "4".into()),
            ],
        };
        let event = AmiEvent::from_raw(&raw).expect("should parse");
        assert_eq!(event.event_name(), "OriginateResponse");
        if let AmiEvent::OriginateResponse {
            response, reason, ..
        } = &event
        {
            assert_eq!(response, "Success");
            assert_eq!(reason, "4");
        } else {
            panic!("expected OriginateResponse");
        }
    }

    #[test]
    fn parse_queue_caller_join_event() {
        let raw = RawAmiMessage {
            headers: vec![
                ("Event".into(), "QueueCallerJoin".into()),
                ("Channel".into(), "PJSIP/300-0003".into()),
                ("Uniqueid".into(), "1111.1".into()),
                ("Queue".into(), "support".into()),
                ("Position".into(), "1".into()),
                ("Count".into(), "3".into()),
            ],
        };
        let event = AmiEvent::from_raw(&raw).expect("should parse");
        assert_eq!(event.event_name(), "QueueCallerJoin");
        if let AmiEvent::QueueCallerJoin {
            queue,
            position,
            count,
            ..
        } = &event
        {
            assert_eq!(queue, "support");
            assert_eq!(*position, 1);
            assert_eq!(*count, 3);
        } else {
            panic!("expected QueueCallerJoin");
        }
    }

    #[test]
    fn parse_security_event() {
        let raw = RawAmiMessage {
            headers: vec![
                ("Event".into(), "FailedACL".into()),
                ("Severity".into(), "Error".into()),
                ("Service".into(), "AMI".into()),
                ("AccountID".into(), "admin".into()),
                ("RemoteAddress".into(), "IPV4/TCP/192.168.1.100/5038".into()),
            ],
        };
        let event = AmiEvent::from_raw(&raw).expect("should parse");
        assert_eq!(event.event_name(), "FailedACL");
        if let AmiEvent::FailedACL {
            service,
            account_id,
            ..
        } = &event
        {
            assert_eq!(service, "AMI");
            assert_eq!(account_id, "admin");
        } else {
            panic!("expected FailedACL");
        }
    }

    #[test]
    fn parse_cdr_event() {
        let raw = RawAmiMessage {
            headers: vec![
                ("Event".into(), "Cdr".into()),
                ("Channel".into(), "PJSIP/100-0001".into()),
                ("Uniqueid".into(), "abc.1".into()),
                ("Destination".into(), "200".into()),
                ("Disposition".into(), "ANSWERED".into()),
                ("Duration".into(), "45".into()),
                ("BillableSeconds".into(), "40".into()),
                ("AccountCode".into(), "acct1".into()),
                ("Source".into(), "100".into()),
                ("DestinationContext".into(), "default".into()),
            ],
        };
        let event = AmiEvent::from_raw(&raw).expect("should parse");
        assert_eq!(event.event_name(), "Cdr");
        if let AmiEvent::Cdr {
            disposition,
            duration,
            billable_seconds,
            ..
        } = &event
        {
            assert_eq!(disposition, "ANSWERED");
            assert_eq!(*duration, 45);
            assert_eq!(*billable_seconds, 40);
        } else {
            panic!("expected Cdr");
        }
    }

    #[test]
    fn parse_confbridge_join_event() {
        let raw = RawAmiMessage {
            headers: vec![
                ("Event".into(), "ConfbridgeJoin".into()),
                ("BridgeUniqueid".into(), "bridge-1".into()),
                ("Conference".into(), "conf-100".into()),
                ("Channel".into(), "PJSIP/100-0001".into()),
                ("Uniqueid".into(), "abc.1".into()),
                ("Admin".into(), "Yes".into()),
            ],
        };
        let event = AmiEvent::from_raw(&raw).expect("should parse");
        assert_eq!(event.event_name(), "ConfbridgeJoin");
        if let AmiEvent::ConfbridgeJoin {
            conference, admin, ..
        } = &event
        {
            assert_eq!(conference, "conf-100");
            assert_eq!(admin, "Yes");
        } else {
            panic!("expected ConfbridgeJoin");
        }
    }
}
