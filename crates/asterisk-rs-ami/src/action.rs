//! typed AMI action types
//!
//! each action serializes to a [`RawAmiMessage`] for transmission

use crate::codec::RawAmiMessage;
use std::sync::atomic::{AtomicU64, Ordering};
use zeroize::Zeroizing;

// relaxed is sufficient: fetch_add is an atomic RMW — it cannot return
// the same value to two threads. no other memory operations need
// ordering relative to this counter
static ACTION_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// generate a unique action ID
pub fn next_action_id() -> String {
    ACTION_ID_COUNTER
        .fetch_add(1, Ordering::Relaxed)
        .to_string()
}

/// trait for types that can be serialized as AMI actions
pub trait AmiAction {
    /// the AMI action name (e.g., "Login", "Originate")
    fn action_name(&self) -> &str;

    /// serialize to key-value pairs (excluding Action and ActionID headers)
    fn to_headers(&self) -> Vec<(String, String)>;

    /// serialize to a complete raw message with Action and ActionID headers
    fn to_message(&self) -> (String, RawAmiMessage) {
        let action_id = next_action_id();
        let mut headers = vec![
            ("Action".to_string(), self.action_name().to_string()),
            ("ActionID".to_string(), action_id.clone()),
        ];
        headers.extend(self.to_headers());
        (
            action_id,
            RawAmiMessage {
                headers,
                output: vec![],
                channel_variables: std::collections::HashMap::new(),
            },
        )
    }
}

/// login with plaintext credentials
pub struct LoginAction {
    pub username: String,
    secret: Zeroizing<String>,
}

impl LoginAction {
    pub fn new(username: impl Into<String>, secret: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            secret: Zeroizing::new(secret.into()),
        }
    }

    pub fn secret(&self) -> &str {
        &self.secret
    }
}

impl std::fmt::Debug for LoginAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginAction")
            .field("username", &self.username)
            .field("secret", &"[REDACTED]")
            .finish()
    }
}

impl AmiAction for LoginAction {
    fn action_name(&self) -> &str {
        "Login"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Username".into(), self.username.clone()),
            ("Secret".into(), self.secret().to_string()),
        ]
    }
}

/// request MD5 challenge
pub struct ChallengeAction;

impl AmiAction for ChallengeAction {
    fn action_name(&self) -> &str {
        "Challenge"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("AuthType".into(), "md5".into())]
    }
}

/// login with MD5 challenge-response
pub struct ChallengeLoginAction {
    pub username: String,
    pub key: Zeroizing<String>,
}

impl std::fmt::Debug for ChallengeLoginAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChallengeLoginAction")
            .field("username", &self.username)
            .field("key", &"[REDACTED]")
            .finish()
    }
}

impl AmiAction for ChallengeLoginAction {
    fn action_name(&self) -> &str {
        "Login"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("AuthType".into(), "md5".into()),
            ("Username".into(), self.username.clone()),
            ("Key".into(), self.key.as_str().to_owned()),
        ]
    }
}

/// logoff from AMI
pub struct LogoffAction;

impl AmiAction for LogoffAction {
    fn action_name(&self) -> &str {
        "Logoff"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// ping the server (keep-alive)
pub struct PingAction;

impl AmiAction for PingAction {
    fn action_name(&self) -> &str {
        "Ping"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// originate a call
pub struct OriginateAction {
    pub channel: String,
    pub context: Option<String>,
    pub exten: Option<String>,
    pub priority: Option<u32>,
    pub application: Option<String>,
    pub data: Option<String>,
    pub timeout: Option<u64>,
    pub caller_id: Option<String>,
    pub account: Option<String>,
    pub async_: bool,
    pub variables: Vec<(String, String)>,
}

impl OriginateAction {
    pub fn new(channel: impl Into<String>) -> Self {
        Self {
            channel: channel.into(),
            context: None,
            exten: None,
            priority: None,
            application: None,
            data: None,
            timeout: None,
            caller_id: None,
            account: None,
            async_: false,
            variables: Vec::new(),
        }
    }

    pub fn context(mut self, ctx: impl Into<String>) -> Self {
        self.context = Some(ctx.into());
        self
    }

    pub fn extension(mut self, ext: impl Into<String>) -> Self {
        self.exten = Some(ext.into());
        self
    }

    pub fn priority(mut self, pri: u32) -> Self {
        self.priority = Some(pri);
        self
    }

    pub fn application(mut self, app: impl Into<String>) -> Self {
        self.application = Some(app.into());
        self
    }

    pub fn data(mut self, data: impl Into<String>) -> Self {
        self.data = Some(data.into());
        self
    }

    pub fn timeout_ms(mut self, ms: u64) -> Self {
        self.timeout = Some(ms);
        self
    }

    pub fn caller_id(mut self, cid: impl Into<String>) -> Self {
        self.caller_id = Some(cid.into());
        self
    }

    pub fn account(mut self, acct: impl Into<String>) -> Self {
        self.account = Some(acct.into());
        self
    }

    pub fn async_originate(mut self, async_: bool) -> Self {
        self.async_ = async_;
        self
    }

    pub fn variable(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.push((key.into(), value.into()));
        self
    }
}

impl AmiAction for OriginateAction {
    fn action_name(&self) -> &str {
        "Originate"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![("Channel".into(), self.channel.clone())];
        if let Some(ref ctx) = self.context {
            h.push(("Context".into(), ctx.clone()));
        }
        if let Some(ref ext) = self.exten {
            h.push(("Exten".into(), ext.clone()));
        }
        if let Some(pri) = self.priority {
            h.push(("Priority".into(), pri.to_string()));
        }
        if let Some(ref app) = self.application {
            h.push(("Application".into(), app.clone()));
        }
        if let Some(ref data) = self.data {
            h.push(("Data".into(), data.clone()));
        }
        if let Some(timeout) = self.timeout {
            h.push(("Timeout".into(), timeout.to_string()));
        }
        if let Some(ref cid) = self.caller_id {
            h.push(("CallerID".into(), cid.clone()));
        }
        if let Some(ref acct) = self.account {
            h.push(("Account".into(), acct.clone()));
        }
        if self.async_ {
            h.push(("Async".into(), "true".into()));
        }
        for (k, v) in &self.variables {
            h.push(("Variable".into(), format!("{k}={v}")));
        }
        h
    }
}

/// hangup a channel
pub struct HangupAction {
    pub channel: String,
    pub cause: Option<u32>,
}

impl HangupAction {
    pub fn new(channel: impl Into<String>) -> Self {
        Self {
            channel: channel.into(),
            cause: None,
        }
    }

    pub fn cause(mut self, cause: u32) -> Self {
        self.cause = Some(cause);
        self
    }
}

impl AmiAction for HangupAction {
    fn action_name(&self) -> &str {
        "Hangup"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![("Channel".into(), self.channel.clone())];
        if let Some(cause) = self.cause {
            h.push(("Cause".into(), cause.to_string()));
        }
        h
    }
}

/// redirect (transfer) a channel
pub struct RedirectAction {
    pub channel: String,
    pub context: String,
    pub exten: String,
    pub priority: u32,
}

impl AmiAction for RedirectAction {
    fn action_name(&self) -> &str {
        "Redirect"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Channel".into(), self.channel.clone()),
            ("Context".into(), self.context.clone()),
            ("Exten".into(), self.exten.clone()),
            ("Priority".into(), self.priority.to_string()),
        ]
    }
}

/// execute a CLI command
pub struct CommandAction {
    pub command: String,
}

impl CommandAction {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
        }
    }
}

impl AmiAction for CommandAction {
    fn action_name(&self) -> &str {
        "Command"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Command".into(), self.command.clone())]
    }
}

/// get a channel variable
pub struct GetVarAction {
    pub channel: Option<String>,
    pub variable: String,
}

impl AmiAction for GetVarAction {
    fn action_name(&self) -> &str {
        "GetVar"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![("Variable".into(), self.variable.clone())];
        if let Some(ref ch) = self.channel {
            h.push(("Channel".into(), ch.clone()));
        }
        h
    }
}

/// set a channel variable
pub struct SetVarAction {
    pub channel: Option<String>,
    pub variable: String,
    pub value: String,
}

impl AmiAction for SetVarAction {
    fn action_name(&self) -> &str {
        "SetVar"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![
            ("Variable".into(), self.variable.clone()),
            ("Value".into(), self.value.clone()),
        ];
        if let Some(ref ch) = self.channel {
            h.push(("Channel".into(), ch.clone()));
        }
        h
    }
}

// ---------------------------------------------------------------------------
// core / system
// ---------------------------------------------------------------------------

/// query channel status
pub struct StatusAction {
    pub channel: Option<String>,
}

impl AmiAction for StatusAction {
    fn action_name(&self) -> &str {
        "Status"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![];
        if let Some(ref ch) = self.channel {
            h.push(("Channel".into(), ch.clone()));
        }
        h
    }
}

/// query core system status
pub struct CoreStatusAction;

impl AmiAction for CoreStatusAction {
    fn action_name(&self) -> &str {
        "CoreStatus"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// query core settings
pub struct CoreSettingsAction;

impl AmiAction for CoreSettingsAction {
    fn action_name(&self) -> &str {
        "CoreSettings"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// list active channels
pub struct CoreShowChannelsAction;

impl AmiAction for CoreShowChannelsAction {
    fn action_name(&self) -> &str {
        "CoreShowChannels"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// show channel map for a given channel
pub struct CoreShowChannelMapAction {
    pub channel: String,
}

impl AmiAction for CoreShowChannelMapAction {
    fn action_name(&self) -> &str {
        "CoreShowChannelMap"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Channel".into(), self.channel.clone())]
    }
}

/// list available AMI commands
pub struct ListCommandsAction;

impl AmiAction for ListCommandsAction {
    fn action_name(&self) -> &str {
        "ListCommands"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// control event output
pub struct EventsAction {
    pub event_mask: String,
}

impl AmiAction for EventsAction {
    fn action_name(&self) -> &str {
        "Events"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("EventMask".into(), self.event_mask.clone())]
    }
}

/// manage event filters
pub struct FilterAction {
    pub operation: String,
    pub filter: Option<String>,
}

impl AmiAction for FilterAction {
    fn action_name(&self) -> &str {
        "Filter"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![("Operation".into(), self.operation.clone())];
        if let Some(ref f) = self.filter {
            h.push(("Filter".into(), f.clone()));
        }
        h
    }
}

/// wait for an event to occur
pub struct WaitEventAction {
    pub timeout: u32,
}

impl AmiAction for WaitEventAction {
    fn action_name(&self) -> &str {
        "WaitEvent"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Timeout".into(), self.timeout.to_string())]
    }
}

/// reload asterisk modules
pub struct ReloadAction {
    pub module: Option<String>,
}

impl AmiAction for ReloadAction {
    fn action_name(&self) -> &str {
        "Reload"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![];
        if let Some(ref m) = self.module {
            h.push(("Module".into(), m.clone()));
        }
        h
    }
}

/// rotate logger files
pub struct LoggerRotateAction;

impl AmiAction for LoggerRotateAction {
    fn action_name(&self) -> &str {
        "LoggerRotate"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// check if a module is loaded
pub struct ModuleCheckAction {
    pub module: String,
}

impl AmiAction for ModuleCheckAction {
    fn action_name(&self) -> &str {
        "ModuleCheck"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Module".into(), self.module.clone())]
    }
}

/// load, unload, or reload a module
pub struct ModuleLoadAction {
    pub module: String,
    pub load_type: String,
}

impl AmiAction for ModuleLoadAction {
    fn action_name(&self) -> &str {
        "ModuleLoad"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Module".into(), self.module.clone()),
            ("LoadType".into(), self.load_type.clone()),
        ]
    }
}

/// send a user-defined event
pub struct UserEventAction {
    pub user_event: String,
    pub headers: Vec<(String, String)>,
}

impl AmiAction for UserEventAction {
    fn action_name(&self) -> &str {
        "UserEvent"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![("UserEvent".into(), self.user_event.clone())];
        for (k, v) in &self.headers {
            h.push((k.clone(), v.clone()));
        }
        h
    }
}

// ---------------------------------------------------------------------------
// channel operations
// ---------------------------------------------------------------------------

/// set absolute timeout on a channel
pub struct AbsoluteTimeoutAction {
    pub channel: String,
    pub timeout: u32,
}

impl AmiAction for AbsoluteTimeoutAction {
    fn action_name(&self) -> &str {
        "AbsoluteTimeout"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Channel".into(), self.channel.clone()),
            ("Timeout".into(), self.timeout.to_string()),
        ]
    }
}

/// mute or unmute audio on a channel
pub struct MuteAudioAction {
    pub channel: String,
    pub direction: String,
    pub state: String,
}

impl AmiAction for MuteAudioAction {
    fn action_name(&self) -> &str {
        "MuteAudio"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Channel".into(), self.channel.clone()),
            ("Direction".into(), self.direction.clone()),
            ("State".into(), self.state.clone()),
        ]
    }
}

/// send text to a channel
pub struct SendTextAction {
    pub channel: String,
    pub message: String,
}

impl AmiAction for SendTextAction {
    fn action_name(&self) -> &str {
        "SendText"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Channel".into(), self.channel.clone()),
            ("Message".into(), self.message.clone()),
        ]
    }
}

/// play a DTMF digit on a channel
pub struct PlayDTMFAction {
    pub channel: String,
    pub digit: String,
    pub duration: Option<u32>,
}

impl AmiAction for PlayDTMFAction {
    fn action_name(&self) -> &str {
        "PlayDTMF"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![
            ("Channel".into(), self.channel.clone()),
            ("Digit".into(), self.digit.clone()),
        ];
        if let Some(d) = self.duration {
            h.push(("Duration".into(), d.to_string()));
        }
        h
    }
}

/// execute an AGI command on a channel
pub struct AGIAction {
    pub channel: String,
    pub command: String,
    pub command_id: Option<String>,
}

impl AmiAction for AGIAction {
    fn action_name(&self) -> &str {
        "AGI"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![
            ("Channel".into(), self.channel.clone()),
            ("Command".into(), self.command.clone()),
        ];
        if let Some(ref id) = self.command_id {
            h.push(("CommandID".into(), id.clone()));
        }
        h
    }
}

// ---------------------------------------------------------------------------
// database
// ---------------------------------------------------------------------------

/// get a value from the asterisk database
pub struct DBGetAction {
    pub family: String,
    pub key: String,
}

impl AmiAction for DBGetAction {
    fn action_name(&self) -> &str {
        "DBGet"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Family".into(), self.family.clone()),
            ("Key".into(), self.key.clone()),
        ]
    }
}

/// put a value into the asterisk database
pub struct DBPutAction {
    pub family: String,
    pub key: String,
    pub val: String,
}

impl AmiAction for DBPutAction {
    fn action_name(&self) -> &str {
        "DBPut"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Family".into(), self.family.clone()),
            ("Key".into(), self.key.clone()),
            ("Val".into(), self.val.clone()),
        ]
    }
}

/// delete a key from the asterisk database
pub struct DBDelAction {
    pub family: String,
    pub key: String,
}

impl AmiAction for DBDelAction {
    fn action_name(&self) -> &str {
        "DBDel"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Family".into(), self.family.clone()),
            ("Key".into(), self.key.clone()),
        ]
    }
}

/// delete a family or subtree from the asterisk database
pub struct DBDelTreeAction {
    pub family: String,
    pub key: Option<String>,
}

impl AmiAction for DBDelTreeAction {
    fn action_name(&self) -> &str {
        "DBDelTree"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![("Family".into(), self.family.clone())];
        if let Some(ref k) = self.key {
            h.push(("Key".into(), k.clone()));
        }
        h
    }
}

/// get a tree of values from the asterisk database
pub struct DBGetTreeAction {
    pub family: String,
    pub key: Option<String>,
}

impl AmiAction for DBGetTreeAction {
    fn action_name(&self) -> &str {
        "DBGetTree"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![("Family".into(), self.family.clone())];
        if let Some(ref k) = self.key {
            h.push(("Key".into(), k.clone()));
        }
        h
    }
}

// ---------------------------------------------------------------------------
// transfer
// ---------------------------------------------------------------------------

/// attended transfer a channel
pub struct AtxferAction {
    pub channel: String,
    pub exten: String,
    pub context: String,
}

impl AmiAction for AtxferAction {
    fn action_name(&self) -> &str {
        "Atxfer"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Channel".into(), self.channel.clone()),
            ("Exten".into(), self.exten.clone()),
            ("Context".into(), self.context.clone()),
        ]
    }
}

/// blind transfer a channel
pub struct BlindTransferAction {
    pub channel: String,
    pub exten: String,
    pub context: String,
}

impl AmiAction for BlindTransferAction {
    fn action_name(&self) -> &str {
        "BlindTransfer"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Channel".into(), self.channel.clone()),
            ("Exten".into(), self.exten.clone()),
            ("Context".into(), self.context.clone()),
        ]
    }
}

/// cancel an attended transfer
pub struct CancelAtxferAction {
    pub channel: String,
}

impl AmiAction for CancelAtxferAction {
    fn action_name(&self) -> &str {
        "CancelAtxfer"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Channel".into(), self.channel.clone())]
    }
}

// ---------------------------------------------------------------------------
// bridge
// ---------------------------------------------------------------------------

/// bridge two channels together
pub struct BridgeAction {
    pub channel1: String,
    pub channel2: String,
    pub tone: Option<String>,
}

impl AmiAction for BridgeAction {
    fn action_name(&self) -> &str {
        "Bridge"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![
            ("Channel1".into(), self.channel1.clone()),
            ("Channel2".into(), self.channel2.clone()),
        ];
        if let Some(ref t) = self.tone {
            h.push(("Tone".into(), t.clone()));
        }
        h
    }
}

/// destroy a bridge
pub struct BridgeDestroyAction {
    pub bridge_unique_id: String,
}

impl AmiAction for BridgeDestroyAction {
    fn action_name(&self) -> &str {
        "BridgeDestroy"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("BridgeUniqueid".into(), self.bridge_unique_id.clone())]
    }
}

/// get information about a bridge
pub struct BridgeInfoAction {
    pub bridge_unique_id: String,
}

impl AmiAction for BridgeInfoAction {
    fn action_name(&self) -> &str {
        "BridgeInfo"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("BridgeUniqueid".into(), self.bridge_unique_id.clone())]
    }
}

/// kick a channel from a bridge
pub struct BridgeKickAction {
    pub bridge_unique_id: String,
    pub channel: String,
}

impl AmiAction for BridgeKickAction {
    fn action_name(&self) -> &str {
        "BridgeKick"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("BridgeUniqueid".into(), self.bridge_unique_id.clone()),
            ("Channel".into(), self.channel.clone()),
        ]
    }
}

/// list active bridges
pub struct BridgeListAction;

impl AmiAction for BridgeListAction {
    fn action_name(&self) -> &str {
        "BridgeList"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

// ---------------------------------------------------------------------------
// queue
// ---------------------------------------------------------------------------

/// add a member to a queue
pub struct QueueAddAction {
    pub queue: String,
    pub interface: String,
    pub penalty: Option<u32>,
    pub paused: Option<bool>,
    pub member_name: Option<String>,
    pub state_interface: Option<String>,
}

impl QueueAddAction {
    pub fn new(queue: impl Into<String>, interface: impl Into<String>) -> Self {
        Self {
            queue: queue.into(),
            interface: interface.into(),
            penalty: None,
            paused: None,
            member_name: None,
            state_interface: None,
        }
    }

    pub fn penalty(mut self, penalty: u32) -> Self {
        self.penalty = Some(penalty);
        self
    }

    pub fn paused(mut self, paused: bool) -> Self {
        self.paused = Some(paused);
        self
    }

    pub fn member_name(mut self, name: impl Into<String>) -> Self {
        self.member_name = Some(name.into());
        self
    }

    pub fn state_interface(mut self, iface: impl Into<String>) -> Self {
        self.state_interface = Some(iface.into());
        self
    }
}

impl AmiAction for QueueAddAction {
    fn action_name(&self) -> &str {
        "QueueAdd"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![
            ("Queue".into(), self.queue.clone()),
            ("Interface".into(), self.interface.clone()),
        ];
        if let Some(p) = self.penalty {
            h.push(("Penalty".into(), p.to_string()));
        }
        if let Some(p) = self.paused {
            h.push(("Paused".into(), p.to_string()));
        }
        if let Some(ref n) = self.member_name {
            h.push(("MemberName".into(), n.clone()));
        }
        if let Some(ref si) = self.state_interface {
            h.push(("StateInterface".into(), si.clone()));
        }
        h
    }
}

/// remove a member from a queue
pub struct QueueRemoveAction {
    pub queue: String,
    pub interface: String,
}

impl AmiAction for QueueRemoveAction {
    fn action_name(&self) -> &str {
        "QueueRemove"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Queue".into(), self.queue.clone()),
            ("Interface".into(), self.interface.clone()),
        ]
    }
}

/// pause or unpause a queue member
pub struct QueuePauseAction {
    pub queue: Option<String>,
    pub interface: String,
    pub paused: bool,
    pub reason: Option<String>,
}

impl AmiAction for QueuePauseAction {
    fn action_name(&self) -> &str {
        "QueuePause"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![
            ("Interface".into(), self.interface.clone()),
            ("Paused".into(), self.paused.to_string()),
        ];
        if let Some(ref q) = self.queue {
            h.push(("Queue".into(), q.clone()));
        }
        if let Some(ref r) = self.reason {
            h.push(("Reason".into(), r.clone()));
        }
        h
    }
}

/// set penalty for a queue member
pub struct QueuePenaltyAction {
    pub interface: String,
    pub penalty: u32,
    pub queue: Option<String>,
}

impl AmiAction for QueuePenaltyAction {
    fn action_name(&self) -> &str {
        "QueuePenalty"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![
            ("Interface".into(), self.interface.clone()),
            ("Penalty".into(), self.penalty.to_string()),
        ];
        if let Some(ref q) = self.queue {
            h.push(("Queue".into(), q.clone()));
        }
        h
    }
}

/// query queue status
pub struct QueueStatusAction {
    pub queue: Option<String>,
    pub member: Option<String>,
}

impl AmiAction for QueueStatusAction {
    fn action_name(&self) -> &str {
        "QueueStatus"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![];
        if let Some(ref q) = self.queue {
            h.push(("Queue".into(), q.clone()));
        }
        if let Some(ref m) = self.member {
            h.push(("Member".into(), m.clone()));
        }
        h
    }
}

/// query queue summary
pub struct QueueSummaryAction {
    pub queue: Option<String>,
}

impl AmiAction for QueueSummaryAction {
    fn action_name(&self) -> &str {
        "QueueSummary"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![];
        if let Some(ref q) = self.queue {
            h.push(("Queue".into(), q.clone()));
        }
        h
    }
}

/// reload queue configuration
pub struct QueueReloadAction {
    pub queue: Option<String>,
    pub members: Option<String>,
    pub rules: Option<String>,
    pub parameters: Option<String>,
}

impl AmiAction for QueueReloadAction {
    fn action_name(&self) -> &str {
        "QueueReload"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![];
        if let Some(ref q) = self.queue {
            h.push(("Queue".into(), q.clone()));
        }
        if let Some(ref m) = self.members {
            h.push(("Members".into(), m.clone()));
        }
        if let Some(ref r) = self.rules {
            h.push(("Rules".into(), r.clone()));
        }
        if let Some(ref p) = self.parameters {
            h.push(("Parameters".into(), p.clone()));
        }
        h
    }
}

/// reset queue statistics
pub struct QueueResetAction {
    pub queue: Option<String>,
}

impl AmiAction for QueueResetAction {
    fn action_name(&self) -> &str {
        "QueueReset"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![];
        if let Some(ref q) = self.queue {
            h.push(("Queue".into(), q.clone()));
        }
        h
    }
}

/// add a custom entry to the queue log
pub struct QueueLogAction {
    pub queue: String,
    pub event: String,
    pub interface: Option<String>,
    pub unique_id: Option<String>,
    pub message: Option<String>,
}

impl AmiAction for QueueLogAction {
    fn action_name(&self) -> &str {
        "QueueLog"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![
            ("Queue".into(), self.queue.clone()),
            ("Event".into(), self.event.clone()),
        ];
        if let Some(ref i) = self.interface {
            h.push(("Interface".into(), i.clone()));
        }
        if let Some(ref u) = self.unique_id {
            h.push(("UniqueID".into(), u.clone()));
        }
        if let Some(ref m) = self.message {
            h.push(("Message".into(), m.clone()));
        }
        h
    }
}

// ---------------------------------------------------------------------------
// mixmonitor
// ---------------------------------------------------------------------------

/// start recording a channel with mixmonitor
pub struct MixMonitorAction {
    pub channel: String,
    pub file: Option<String>,
    pub options: Option<String>,
}

impl MixMonitorAction {
    pub fn new(channel: impl Into<String>) -> Self {
        Self {
            channel: channel.into(),
            file: None,
            options: None,
        }
    }

    pub fn file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }

    pub fn options(mut self, opts: impl Into<String>) -> Self {
        self.options = Some(opts.into());
        self
    }
}

impl AmiAction for MixMonitorAction {
    fn action_name(&self) -> &str {
        "MixMonitor"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![("Channel".into(), self.channel.clone())];
        if let Some(ref f) = self.file {
            h.push(("File".into(), f.clone()));
        }
        if let Some(ref o) = self.options {
            h.push(("Options".into(), o.clone()));
        }
        h
    }
}

/// mute or unmute a mixmonitor recording
pub struct MixMonitorMuteAction {
    pub channel: String,
    pub direction: String,
    pub state: String,
}

impl AmiAction for MixMonitorMuteAction {
    fn action_name(&self) -> &str {
        "MixMonitorMute"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Channel".into(), self.channel.clone()),
            ("Direction".into(), self.direction.clone()),
            ("State".into(), self.state.clone()),
        ]
    }
}

/// stop recording a channel with mixmonitor
pub struct StopMixMonitorAction {
    pub channel: String,
    pub mix_monitor_id: Option<String>,
}

impl AmiAction for StopMixMonitorAction {
    fn action_name(&self) -> &str {
        "StopMixMonitor"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![("Channel".into(), self.channel.clone())];
        if let Some(ref id) = self.mix_monitor_id {
            h.push(("MixMonitorID".into(), id.clone()));
        }
        h
    }
}

/// control playback on a channel
pub struct ControlPlaybackAction {
    pub channel: String,
    pub control: String,
}

impl AmiAction for ControlPlaybackAction {
    fn action_name(&self) -> &str {
        "ControlPlayback"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Channel".into(), self.channel.clone()),
            ("Control".into(), self.control.clone()),
        ]
    }
}

// ---------------------------------------------------------------------------
// confbridge
// ---------------------------------------------------------------------------

/// list participants in a conference
pub struct ConfbridgeListAction {
    pub conference: String,
}

impl AmiAction for ConfbridgeListAction {
    fn action_name(&self) -> &str {
        "ConfbridgeList"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Conference".into(), self.conference.clone())]
    }
}

/// list active conference rooms
pub struct ConfbridgeListRoomsAction;

impl AmiAction for ConfbridgeListRoomsAction {
    fn action_name(&self) -> &str {
        "ConfbridgeListRooms"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// kick a participant from a conference
pub struct ConfbridgeKickAction {
    pub conference: String,
    pub channel: String,
}

impl AmiAction for ConfbridgeKickAction {
    fn action_name(&self) -> &str {
        "ConfbridgeKick"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Conference".into(), self.conference.clone()),
            ("Channel".into(), self.channel.clone()),
        ]
    }
}

/// mute a participant in a conference
pub struct ConfbridgeMuteAction {
    pub conference: String,
    pub channel: String,
}

impl AmiAction for ConfbridgeMuteAction {
    fn action_name(&self) -> &str {
        "ConfbridgeMute"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Conference".into(), self.conference.clone()),
            ("Channel".into(), self.channel.clone()),
        ]
    }
}

/// unmute a participant in a conference
pub struct ConfbridgeUnmuteAction {
    pub conference: String,
    pub channel: String,
}

impl AmiAction for ConfbridgeUnmuteAction {
    fn action_name(&self) -> &str {
        "ConfbridgeUnmute"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Conference".into(), self.conference.clone()),
            ("Channel".into(), self.channel.clone()),
        ]
    }
}

/// lock a conference
pub struct ConfbridgeLockAction {
    pub conference: String,
}

impl AmiAction for ConfbridgeLockAction {
    fn action_name(&self) -> &str {
        "ConfbridgeLock"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Conference".into(), self.conference.clone())]
    }
}

/// unlock a conference
pub struct ConfbridgeUnlockAction {
    pub conference: String,
}

impl AmiAction for ConfbridgeUnlockAction {
    fn action_name(&self) -> &str {
        "ConfbridgeUnlock"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Conference".into(), self.conference.clone())]
    }
}

/// start recording a conference
pub struct ConfbridgeStartRecordAction {
    pub conference: String,
    pub record_file: Option<String>,
}

impl AmiAction for ConfbridgeStartRecordAction {
    fn action_name(&self) -> &str {
        "ConfbridgeStartRecord"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![("Conference".into(), self.conference.clone())];
        if let Some(ref f) = self.record_file {
            h.push(("RecordFile".into(), f.clone()));
        }
        h
    }
}

/// stop recording a conference
pub struct ConfbridgeStopRecordAction {
    pub conference: String,
}

impl AmiAction for ConfbridgeStopRecordAction {
    fn action_name(&self) -> &str {
        "ConfbridgeStopRecord"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Conference".into(), self.conference.clone())]
    }
}

// ---------------------------------------------------------------------------
// parking
// ---------------------------------------------------------------------------

/// park a channel
pub struct ParkAction {
    pub channel: String,
    pub timeout: Option<u32>,
    pub announce_channel: Option<String>,
    pub parking_lot: Option<String>,
}

impl ParkAction {
    pub fn new(channel: impl Into<String>) -> Self {
        Self {
            channel: channel.into(),
            timeout: None,
            announce_channel: None,
            parking_lot: None,
        }
    }

    pub fn timeout(mut self, timeout: u32) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn announce_channel(mut self, ch: impl Into<String>) -> Self {
        self.announce_channel = Some(ch.into());
        self
    }

    pub fn parking_lot(mut self, lot: impl Into<String>) -> Self {
        self.parking_lot = Some(lot.into());
        self
    }
}

impl AmiAction for ParkAction {
    fn action_name(&self) -> &str {
        "Park"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![("Channel".into(), self.channel.clone())];
        if let Some(t) = self.timeout {
            h.push(("Timeout".into(), t.to_string()));
        }
        if let Some(ref a) = self.announce_channel {
            h.push(("AnnounceChannel".into(), a.clone()));
        }
        if let Some(ref l) = self.parking_lot {
            h.push(("ParkingLot".into(), l.clone()));
        }
        h
    }
}

/// list parked calls
pub struct ParkedCallsAction {
    pub parking_lot: Option<String>,
}

impl AmiAction for ParkedCallsAction {
    fn action_name(&self) -> &str {
        "ParkedCalls"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![];
        if let Some(ref l) = self.parking_lot {
            h.push(("ParkingLot".into(), l.clone()));
        }
        h
    }
}

/// list parking lots
pub struct ParkinglotsAction;

impl AmiAction for ParkinglotsAction {
    fn action_name(&self) -> &str {
        "Parkinglots"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

// ---------------------------------------------------------------------------
// config
// ---------------------------------------------------------------------------

/// retrieve configuration file
pub struct GetConfigAction {
    pub filename: String,
    pub category: Option<String>,
}

impl AmiAction for GetConfigAction {
    fn action_name(&self) -> &str {
        "GetConfig"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![("Filename".into(), self.filename.clone())];
        if let Some(ref c) = self.category {
            h.push(("Category".into(), c.clone()));
        }
        h
    }
}

/// retrieve configuration as JSON
pub struct GetConfigJSONAction {
    pub filename: String,
}

impl AmiAction for GetConfigJSONAction {
    fn action_name(&self) -> &str {
        "GetConfigJSON"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Filename".into(), self.filename.clone())]
    }
}

/// update a configuration file
pub struct UpdateConfigAction {
    pub src_filename: String,
    pub dst_filename: String,
    pub reload: Option<String>,
    pub actions: Vec<(String, String)>,
}

impl AmiAction for UpdateConfigAction {
    fn action_name(&self) -> &str {
        "UpdateConfig"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![
            ("SrcFilename".into(), self.src_filename.clone()),
            ("DstFilename".into(), self.dst_filename.clone()),
        ];
        if let Some(ref r) = self.reload {
            h.push(("Reload".into(), r.clone()));
        }
        for (i, (k, v)) in self.actions.iter().enumerate() {
            h.push((format!("Action-{i:06}"), k.clone()));
            h.push((format!("Cat-{i:06}"), v.clone()));
        }
        h
    }
}

/// create an empty configuration file
pub struct CreateConfigAction {
    pub filename: String,
}

impl AmiAction for CreateConfigAction {
    fn action_name(&self) -> &str {
        "CreateConfig"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Filename".into(), self.filename.clone())]
    }
}

/// list categories in a configuration file
pub struct ListCategoriesAction {
    pub filename: String,
}

impl AmiAction for ListCategoriesAction {
    fn action_name(&self) -> &str {
        "ListCategories"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Filename".into(), self.filename.clone())]
    }
}

/// show dialplan
pub struct ShowDialPlanAction {
    pub extension: Option<String>,
    pub context: Option<String>,
}

impl AmiAction for ShowDialPlanAction {
    fn action_name(&self) -> &str {
        "ShowDialPlan"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![];
        if let Some(ref e) = self.extension {
            h.push(("Extension".into(), e.clone()));
        }
        if let Some(ref c) = self.context {
            h.push(("Context".into(), c.clone()));
        }
        h
    }
}

// ---------------------------------------------------------------------------
// pjsip
// ---------------------------------------------------------------------------

/// list all pjsip endpoints
pub struct PJSIPShowEndpointsAction;

impl AmiAction for PJSIPShowEndpointsAction {
    fn action_name(&self) -> &str {
        "PJSIPShowEndpoints"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// show details for a pjsip endpoint
pub struct PJSIPShowEndpointAction {
    pub endpoint: String,
}

impl AmiAction for PJSIPShowEndpointAction {
    fn action_name(&self) -> &str {
        "PJSIPShowEndpoint"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Endpoint".into(), self.endpoint.clone())]
    }
}

/// qualify a pjsip endpoint
pub struct PJSIPQualifyAction {
    pub endpoint: String,
}

impl AmiAction for PJSIPQualifyAction {
    fn action_name(&self) -> &str {
        "PJSIPQualify"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Endpoint".into(), self.endpoint.clone())]
    }
}

/// register a pjsip outbound registration
pub struct PJSIPRegisterAction {
    pub registration: String,
}

impl AmiAction for PJSIPRegisterAction {
    fn action_name(&self) -> &str {
        "PJSIPRegister"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Registration".into(), self.registration.clone())]
    }
}

/// unregister a pjsip outbound registration
pub struct PJSIPUnregisterAction {
    pub registration: String,
}

impl AmiAction for PJSIPUnregisterAction {
    fn action_name(&self) -> &str {
        "PJSIPUnregister"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Registration".into(), self.registration.clone())]
    }
}

/// list inbound pjsip registrations
pub struct PJSIPShowRegistrationsInboundAction;

impl AmiAction for PJSIPShowRegistrationsInboundAction {
    fn action_name(&self) -> &str {
        "PJSIPShowRegistrationsInbound"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// list outbound pjsip registrations
pub struct PJSIPShowRegistrationsOutboundAction;

impl AmiAction for PJSIPShowRegistrationsOutboundAction {
    fn action_name(&self) -> &str {
        "PJSIPShowRegistrationsOutbound"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// list pjsip contacts
pub struct PJSIPShowContactsAction;

impl AmiAction for PJSIPShowContactsAction {
    fn action_name(&self) -> &str {
        "PJSIPShowContacts"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// list pjsip address of records
pub struct PJSIPShowAorsAction;

impl AmiAction for PJSIPShowAorsAction {
    fn action_name(&self) -> &str {
        "PJSIPShowAors"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// list pjsip authentication objects
pub struct PJSIPShowAuthsAction;

impl AmiAction for PJSIPShowAuthsAction {
    fn action_name(&self) -> &str {
        "PJSIPShowAuths"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// send a notify to a pjsip endpoint
pub struct PJSIPNotifyAction {
    pub endpoint: String,
    pub variable: Vec<(String, String)>,
}

impl AmiAction for PJSIPNotifyAction {
    fn action_name(&self) -> &str {
        "PJSIPNotify"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![("Endpoint".into(), self.endpoint.clone())];
        for (k, v) in &self.variable {
            h.push(("Variable".into(), format!("{k}={v}")));
        }
        h
    }
}

/// hangup a pjsip channel
pub struct PJSIPHangupAction {
    pub channel: String,
    pub cause: Option<u32>,
}

impl AmiAction for PJSIPHangupAction {
    fn action_name(&self) -> &str {
        "PJSIPHangup"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![("Channel".into(), self.channel.clone())];
        if let Some(c) = self.cause {
            h.push(("Cause".into(), c.to_string()));
        }
        h
    }
}

// ---------------------------------------------------------------------------
// extension / device / presence state
// ---------------------------------------------------------------------------

/// query extension state
pub struct ExtensionStateAction {
    pub exten: String,
    pub context: String,
}

impl AmiAction for ExtensionStateAction {
    fn action_name(&self) -> &str {
        "ExtensionState"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Exten".into(), self.exten.clone()),
            ("Context".into(), self.context.clone()),
        ]
    }
}

/// list all extension states
pub struct ExtensionStateListAction;

impl AmiAction for ExtensionStateListAction {
    fn action_name(&self) -> &str {
        "ExtensionStateList"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// list all device states
pub struct DeviceStateListAction;

impl AmiAction for DeviceStateListAction {
    fn action_name(&self) -> &str {
        "DeviceStateList"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// query presence state for a provider
pub struct PresenceStateAction {
    pub provider: String,
}

impl AmiAction for PresenceStateAction {
    fn action_name(&self) -> &str {
        "PresenceState"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Provider".into(), self.provider.clone())]
    }
}

/// list all presence states
pub struct PresenceStateListAction;

impl AmiAction for PresenceStateListAction {
    fn action_name(&self) -> &str {
        "PresenceStateList"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

// ---------------------------------------------------------------------------
// dialplan
// ---------------------------------------------------------------------------

/// add an extension to the dialplan
pub struct DialplanExtensionAddAction {
    pub context: String,
    pub extension: String,
    pub priority: String,
    pub application: String,
}

impl AmiAction for DialplanExtensionAddAction {
    fn action_name(&self) -> &str {
        "DialplanExtensionAdd"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Context".into(), self.context.clone()),
            ("Extension".into(), self.extension.clone()),
            ("Priority".into(), self.priority.clone()),
            ("Application".into(), self.application.clone()),
        ]
    }
}

/// remove an extension from the dialplan
pub struct DialplanExtensionRemoveAction {
    pub context: String,
    pub extension: String,
    pub priority: Option<String>,
}

impl AmiAction for DialplanExtensionRemoveAction {
    fn action_name(&self) -> &str {
        "DialplanExtensionRemove"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![
            ("Context".into(), self.context.clone()),
            ("Extension".into(), self.extension.clone()),
        ];
        if let Some(ref p) = self.priority {
            h.push(("Priority".into(), p.clone()));
        }
        h
    }
}

/// request local channel optimization
pub struct LocalOptimizeAwayAction {
    pub channel: String,
}

impl AmiAction for LocalOptimizeAwayAction {
    fn action_name(&self) -> &str {
        "LocalOptimizeAway"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Channel".into(), self.channel.clone())]
    }
}

// ---------------------------------------------------------------------------
// mailbox / mwi
// ---------------------------------------------------------------------------

/// get mailbox message count
pub struct MailboxCountAction {
    pub mailbox: String,
}

impl AmiAction for MailboxCountAction {
    fn action_name(&self) -> &str {
        "MailboxCount"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Mailbox".into(), self.mailbox.clone())]
    }
}

/// get mailbox status
pub struct MailboxStatusAction {
    pub mailbox: String,
}

impl AmiAction for MailboxStatusAction {
    fn action_name(&self) -> &str {
        "MailboxStatus"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Mailbox".into(), self.mailbox.clone())]
    }
}

/// get message waiting indicator state
pub struct MWIGetAction {
    pub mailbox: String,
}

impl AmiAction for MWIGetAction {
    fn action_name(&self) -> &str {
        "MWIGet"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Mailbox".into(), self.mailbox.clone())]
    }
}

/// update message waiting indicator
pub struct MWIUpdateAction {
    pub mailbox: String,
    pub old_messages: Option<u32>,
    pub new_messages: Option<u32>,
}

impl AmiAction for MWIUpdateAction {
    fn action_name(&self) -> &str {
        "MWIUpdate"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![("Mailbox".into(), self.mailbox.clone())];
        if let Some(o) = self.old_messages {
            h.push(("OldMessages".into(), o.to_string()));
        }
        if let Some(n) = self.new_messages {
            h.push(("NewMessages".into(), n.to_string()));
        }
        h
    }
}

/// delete message waiting indicator
pub struct MWIDeleteAction {
    pub mailbox: String,
}

impl AmiAction for MWIDeleteAction {
    fn action_name(&self) -> &str {
        "MWIDelete"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Mailbox".into(), self.mailbox.clone())]
    }
}

/// send a text message
pub struct MessageSendAction {
    pub to: String,
    pub from: Option<String>,
    pub body: Option<String>,
}

impl AmiAction for MessageSendAction {
    fn action_name(&self) -> &str {
        "MessageSend"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![("To".into(), self.to.clone())];
        if let Some(ref f) = self.from {
            h.push(("From".into(), f.clone()));
        }
        if let Some(ref b) = self.body {
            h.push(("Body".into(), b.clone()));
        }
        h
    }
}

// ---------------------------------------------------------------------------
// voicemail
// ---------------------------------------------------------------------------

/// list voicemail users
pub struct VoicemailUsersListAction;

impl AmiAction for VoicemailUsersListAction {
    fn action_name(&self) -> &str {
        "VoicemailUsersList"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// get voicemail user status
pub struct VoicemailUserStatusAction {
    pub context: String,
    pub mailbox: String,
}

impl AmiAction for VoicemailUserStatusAction {
    fn action_name(&self) -> &str {
        "VoicemailUserStatus"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Context".into(), self.context.clone()),
            ("Mailbox".into(), self.mailbox.clone()),
        ]
    }
}

/// refresh voicemail state
pub struct VoicemailRefreshAction {
    pub context: Option<String>,
    pub mailbox: Option<String>,
}

impl AmiAction for VoicemailRefreshAction {
    fn action_name(&self) -> &str {
        "VoicemailRefresh"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![];
        if let Some(ref c) = self.context {
            h.push(("Context".into(), c.clone()));
        }
        if let Some(ref m) = self.mailbox {
            h.push(("Mailbox".into(), m.clone()));
        }
        h
    }
}

/// get voicemail box summary
pub struct VoicemailBoxSummaryAction {
    pub context: String,
    pub mailbox: String,
}

impl AmiAction for VoicemailBoxSummaryAction {
    fn action_name(&self) -> &str {
        "VoicemailBoxSummary"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Context".into(), self.context.clone()),
            ("Mailbox".into(), self.mailbox.clone()),
        ]
    }
}

// ---------------------------------------------------------------------------
// meetme
// ---------------------------------------------------------------------------

/// list meetme conference participants
pub struct MeetmeListAction {
    pub conference: Option<String>,
}

impl AmiAction for MeetmeListAction {
    fn action_name(&self) -> &str {
        "MeetmeList"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![];
        if let Some(ref c) = self.conference {
            h.push(("Conference".into(), c.clone()));
        }
        h
    }
}

/// list active meetme rooms
pub struct MeetmeListRoomsAction;

impl AmiAction for MeetmeListRoomsAction {
    fn action_name(&self) -> &str {
        "MeetmeListRooms"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// mute a meetme participant
pub struct MeetmeMuteAction {
    pub meetme: String,
    pub usernum: String,
}

impl AmiAction for MeetmeMuteAction {
    fn action_name(&self) -> &str {
        "MeetmeMute"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Meetme".into(), self.meetme.clone()),
            ("Usernum".into(), self.usernum.clone()),
        ]
    }
}

/// unmute a meetme participant
pub struct MeetmeUnmuteAction {
    pub meetme: String,
    pub usernum: String,
}

impl AmiAction for MeetmeUnmuteAction {
    fn action_name(&self) -> &str {
        "MeetmeUnmute"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Meetme".into(), self.meetme.clone()),
            ("Usernum".into(), self.usernum.clone()),
        ]
    }
}

// ---------------------------------------------------------------------------
// agent
// ---------------------------------------------------------------------------

/// log off an agent
pub struct AgentLogoffAction {
    pub agent: String,
    pub soft: Option<bool>,
}

impl AmiAction for AgentLogoffAction {
    fn action_name(&self) -> &str {
        "AgentLogoff"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![("Agent".into(), self.agent.clone())];
        if let Some(s) = self.soft {
            h.push(("Soft".into(), s.to_string()));
        }
        h
    }
}

/// list agents
pub struct AgentsAction;

impl AmiAction for AgentsAction {
    fn action_name(&self) -> &str {
        "Agents"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

// ---------------------------------------------------------------------------
// fax
// ---------------------------------------------------------------------------

/// get info about a fax session
pub struct FAXSessionAction {
    pub session_number: String,
}

impl AmiAction for FAXSessionAction {
    fn action_name(&self) -> &str {
        "FAXSession"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("SessionNumber".into(), self.session_number.clone())]
    }
}

/// list active fax sessions
pub struct FAXSessionsAction;

impl AmiAction for FAXSessionsAction {
    fn action_name(&self) -> &str {
        "FAXSessions"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// get fax statistics
pub struct FAXStatsAction;

impl AmiAction for FAXStatsAction {
    fn action_name(&self) -> &str {
        "FAXStats"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

// ---------------------------------------------------------------------------
// other
// ---------------------------------------------------------------------------

/// send an advice of charge message
pub struct AOCMessageAction {
    pub channel: String,
    pub msg_type: String,
    pub charge_type: String,
}

impl AmiAction for AOCMessageAction {
    fn action_name(&self) -> &str {
        "AOCMessage"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Channel".into(), self.channel.clone()),
            ("MsgType".into(), self.msg_type.clone()),
            ("ChargeType".into(), self.charge_type.clone()),
        ]
    }
}

/// send a flash signal on a channel
pub struct SendFlashAction {
    pub channel: String,
}

impl AmiAction for SendFlashAction {
    fn action_name(&self) -> &str {
        "SendFlash"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Channel".into(), self.channel.clone())]
    }
}

/// play an MF digit on a channel
pub struct PlayMFAction {
    pub channel: String,
    pub digit: String,
    pub duration: Option<u32>,
}

impl AmiAction for PlayMFAction {
    fn action_name(&self) -> &str {
        "PlayMF"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![
            ("Channel".into(), self.channel.clone()),
            ("Digit".into(), self.digit.clone()),
        ];
        if let Some(d) = self.duration {
            h.push(("Duration".into(), d.to_string()));
        }
        h
    }
}

// ---------------------------------------------------------------------------
// dahdi
// ---------------------------------------------------------------------------

/// disable do not disturb on a DAHDI channel
pub struct DAHDIDNDoffAction {
    pub dahdi_channel: String,
}

impl AmiAction for DAHDIDNDoffAction {
    fn action_name(&self) -> &str {
        "DAHDIDNDoff"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("DAHDIChannel".into(), self.dahdi_channel.clone())]
    }
}

/// enable do not disturb on a DAHDI channel
pub struct DAHDIDNDonAction {
    pub dahdi_channel: String,
}

impl AmiAction for DAHDIDNDonAction {
    fn action_name(&self) -> &str {
        "DAHDIDNDon"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("DAHDIChannel".into(), self.dahdi_channel.clone())]
    }
}

/// dial a number on a DAHDI channel that is off hook
pub struct DAHDIDialOffhookAction {
    pub dahdi_channel: String,
    pub number: String,
}

impl AmiAction for DAHDIDialOffhookAction {
    fn action_name(&self) -> &str {
        "DAHDIDialOffhook"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("DAHDIChannel".into(), self.dahdi_channel.clone()),
            ("Number".into(), self.number.clone()),
        ]
    }
}

/// hangup a DAHDI channel
pub struct DAHDIHangupAction {
    pub dahdi_channel: String,
}

impl AmiAction for DAHDIHangupAction {
    fn action_name(&self) -> &str {
        "DAHDIHangup"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("DAHDIChannel".into(), self.dahdi_channel.clone())]
    }
}

/// restart the DAHDI channels
pub struct DAHDIRestartAction;

impl AmiAction for DAHDIRestartAction {
    fn action_name(&self) -> &str {
        "DAHDIRestart"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// show DAHDI channel information
pub struct DAHDIShowChannelsAction {
    pub dahdi_channel: Option<String>,
}

impl AmiAction for DAHDIShowChannelsAction {
    fn action_name(&self) -> &str {
        "DAHDIShowChannels"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![];
        if let Some(ref ch) = self.dahdi_channel {
            h.push(("DAHDIChannel".into(), ch.clone()));
        }
        h
    }
}

/// show DAHDI status
pub struct DAHDIShowStatusAction;

impl AmiAction for DAHDIShowStatusAction {
    fn action_name(&self) -> &str {
        "DAHDIShowStatus"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// transfer a DAHDI channel
pub struct DAHDITransferAction {
    pub dahdi_channel: String,
}

impl AmiAction for DAHDITransferAction {
    fn action_name(&self) -> &str {
        "DAHDITransfer"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("DAHDIChannel".into(), self.dahdi_channel.clone())]
    }
}

// ---------------------------------------------------------------------------
// iax
// ---------------------------------------------------------------------------

/// show IAX2 network statistics
pub struct IAXnetstatsAction;

impl AmiAction for IAXnetstatsAction {
    fn action_name(&self) -> &str {
        "IAXnetstats"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// list IAX2 peers
pub struct IAXpeerlistAction;

impl AmiAction for IAXpeerlistAction {
    fn action_name(&self) -> &str {
        "IAXpeerlist"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// list IAX2 peers (compact)
pub struct IAXpeersAction;

impl AmiAction for IAXpeersAction {
    fn action_name(&self) -> &str {
        "IAXpeers"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// list IAX2 registrations
pub struct IAXregistryAction;

impl AmiAction for IAXregistryAction {
    fn action_name(&self) -> &str {
        "IAXregistry"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

// ---------------------------------------------------------------------------
// pri
// ---------------------------------------------------------------------------

/// set the PRI debug log file
pub struct PRIDebugFileSetAction {
    pub filename: String,
}

impl AmiAction for PRIDebugFileSetAction {
    fn action_name(&self) -> &str {
        "PRIDebugFileSet"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Filename".into(), self.filename.clone())]
    }
}

/// unset the PRI debug log file
pub struct PRIDebugFileUnsetAction;

impl AmiAction for PRIDebugFileUnsetAction {
    fn action_name(&self) -> &str {
        "PRIDebugFileUnset"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// set the PRI debug level for a span
pub struct PRIDebugSetAction {
    pub span: u32,
    pub level: u32,
}

impl AmiAction for PRIDebugSetAction {
    fn action_name(&self) -> &str {
        "PRIDebugSet"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Span".into(), self.span.to_string()),
            ("Level".into(), self.level.to_string()),
        ]
    }
}

/// show PRI spans
pub struct PRIShowSpansAction {
    pub span: Option<u32>,
}

impl AmiAction for PRIShowSpansAction {
    fn action_name(&self) -> &str {
        "PRIShowSpans"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        let mut h = vec![];
        if let Some(s) = self.span {
            h.push(("Span".into(), s.to_string()));
        }
        h
    }
}

// ---------------------------------------------------------------------------
// bridge technology
// ---------------------------------------------------------------------------

/// list available bridge technologies
pub struct BridgeTechnologyListAction;

impl AmiAction for BridgeTechnologyListAction {
    fn action_name(&self) -> &str {
        "BridgeTechnologyList"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// suspend a bridge technology
pub struct BridgeTechnologySuspendAction {
    pub bridge_technology: String,
}

impl AmiAction for BridgeTechnologySuspendAction {
    fn action_name(&self) -> &str {
        "BridgeTechnologySuspend"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("BridgeTechnology".into(), self.bridge_technology.clone())]
    }
}

/// unsuspend a bridge technology
pub struct BridgeTechnologyUnsuspendAction {
    pub bridge_technology: String,
}

impl AmiAction for BridgeTechnologyUnsuspendAction {
    fn action_name(&self) -> &str {
        "BridgeTechnologyUnsuspend"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("BridgeTechnology".into(), self.bridge_technology.clone())]
    }
}

// ---------------------------------------------------------------------------
// pjsip advanced
// ---------------------------------------------------------------------------

/// show PJSIP inbound registration contact statuses
pub struct PJSIPShowRegistrationInboundContactStatusesAction;

impl AmiAction for PJSIPShowRegistrationInboundContactStatusesAction {
    fn action_name(&self) -> &str {
        "PJSIPShowRegistrationInboundContactStatuses"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// show PJSIP resource lists
pub struct PJSIPShowResourceListsAction;

impl AmiAction for PJSIPShowResourceListsAction {
    fn action_name(&self) -> &str {
        "PJSIPShowResourceLists"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// show PJSIP inbound subscriptions
pub struct PJSIPShowSubscriptionsInboundAction;

impl AmiAction for PJSIPShowSubscriptionsInboundAction {
    fn action_name(&self) -> &str {
        "PJSIPShowSubscriptionsInbound"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// show PJSIP outbound subscriptions
pub struct PJSIPShowSubscriptionsOutboundAction;

impl AmiAction for PJSIPShowSubscriptionsOutboundAction {
    fn action_name(&self) -> &str {
        "PJSIPShowSubscriptionsOutbound"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

// ---------------------------------------------------------------------------
// queue advanced
// ---------------------------------------------------------------------------

/// change priority of a caller in a queue
pub struct QueueChangePriorityCallerAction {
    pub queue: String,
    pub caller: String,
    pub priority: u32,
}

impl AmiAction for QueueChangePriorityCallerAction {
    fn action_name(&self) -> &str {
        "QueueChangePriorityCaller"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Queue".into(), self.queue.clone()),
            ("Caller".into(), self.caller.clone()),
            ("Priority".into(), self.priority.to_string()),
        ]
    }
}

/// set ring in use for a queue member
pub struct QueueMemberRingInUseAction {
    pub interface: String,
    pub ring_in_use: bool,
}

impl AmiAction for QueueMemberRingInUseAction {
    fn action_name(&self) -> &str {
        "QueueMemberRingInUse"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Interface".into(), self.interface.clone()),
            ("RingInUse".into(), self.ring_in_use.to_string()),
        ]
    }
}

/// show a queue rule
pub struct QueueRuleAction {
    pub rule: String,
}

impl AmiAction for QueueRuleAction {
    fn action_name(&self) -> &str {
        "QueueRule"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Rule".into(), self.rule.clone())]
    }
}

/// withdraw a caller from a queue
pub struct QueueWithdrawCallerAction {
    pub queue: String,
    pub caller: String,
}

impl AmiAction for QueueWithdrawCallerAction {
    fn action_name(&self) -> &str {
        "QueueWithdrawCaller"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Queue".into(), self.queue.clone()),
            ("Caller".into(), self.caller.clone()),
        ]
    }
}

// ---------------------------------------------------------------------------
// sorcery cache
// ---------------------------------------------------------------------------

/// expire all objects in a sorcery memory cache
pub struct SorceryMemoryCacheExpireAction {
    pub cache: String,
}

impl AmiAction for SorceryMemoryCacheExpireAction {
    fn action_name(&self) -> &str {
        "SorceryMemoryCacheExpire"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Cache".into(), self.cache.clone())]
    }
}

/// expire a specific object in a sorcery memory cache
pub struct SorceryMemoryCacheExpireObjectAction {
    pub cache: String,
    pub object: String,
}

impl AmiAction for SorceryMemoryCacheExpireObjectAction {
    fn action_name(&self) -> &str {
        "SorceryMemoryCacheExpireObject"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Cache".into(), self.cache.clone()),
            ("Object".into(), self.object.clone()),
        ]
    }
}

/// populate a sorcery memory cache
pub struct SorceryMemoryCachePopulateAction {
    pub cache: String,
}

impl AmiAction for SorceryMemoryCachePopulateAction {
    fn action_name(&self) -> &str {
        "SorceryMemoryCachePopulate"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Cache".into(), self.cache.clone())]
    }
}

/// mark all objects in a sorcery memory cache as stale
pub struct SorceryMemoryCacheStaleAction {
    pub cache: String,
}

impl AmiAction for SorceryMemoryCacheStaleAction {
    fn action_name(&self) -> &str {
        "SorceryMemoryCacheStale"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![("Cache".into(), self.cache.clone())]
    }
}

/// mark a specific object in a sorcery memory cache as stale
pub struct SorceryMemoryCacheStaleObjectAction {
    pub cache: String,
    pub object: String,
}

impl AmiAction for SorceryMemoryCacheStaleObjectAction {
    fn action_name(&self) -> &str {
        "SorceryMemoryCacheStaleObject"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Cache".into(), self.cache.clone()),
            ("Object".into(), self.object.clone()),
        ]
    }
}

// ---------------------------------------------------------------------------
// voicemail
// ---------------------------------------------------------------------------

/// forward a voicemail message
pub struct VoicemailForwardAction {
    pub mailbox: String,
    pub context: String,
    pub from_mailbox: String,
    pub from_context: String,
    pub from_folder: String,
    pub message_id: String,
}

impl AmiAction for VoicemailForwardAction {
    fn action_name(&self) -> &str {
        "VoicemailForward"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Mailbox".into(), self.mailbox.clone()),
            ("Context".into(), self.context.clone()),
            ("FromMailbox".into(), self.from_mailbox.clone()),
            ("FromContext".into(), self.from_context.clone()),
            ("FromFolder".into(), self.from_folder.clone()),
            ("ID".into(), self.message_id.clone()),
        ]
    }
}

/// move a voicemail message
pub struct VoicemailMoveAction {
    pub mailbox: String,
    pub context: String,
    pub folder: String,
    pub message_id: String,
}

impl AmiAction for VoicemailMoveAction {
    fn action_name(&self) -> &str {
        "VoicemailMove"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Mailbox".into(), self.mailbox.clone()),
            ("Context".into(), self.context.clone()),
            ("Folder".into(), self.folder.clone()),
            ("ID".into(), self.message_id.clone()),
        ]
    }
}

/// remove a voicemail message
pub struct VoicemailRemoveAction {
    pub mailbox: String,
    pub context: String,
    pub folder: String,
    pub message_id: String,
}

impl AmiAction for VoicemailRemoveAction {
    fn action_name(&self) -> &str {
        "VoicemailRemove"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Mailbox".into(), self.mailbox.clone()),
            ("Context".into(), self.context.clone()),
            ("Folder".into(), self.folder.clone()),
            ("ID".into(), self.message_id.clone()),
        ]
    }
}

// ---------------------------------------------------------------------------
// confbridge
// ---------------------------------------------------------------------------

/// set the single video source in a conference bridge
pub struct ConfbridgeSetSingleVideoSrcAction {
    pub conference: String,
    pub channel: String,
}

impl AmiAction for ConfbridgeSetSingleVideoSrcAction {
    fn action_name(&self) -> &str {
        "ConfbridgeSetSingleVideoSrc"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Conference".into(), self.conference.clone()),
            ("Channel".into(), self.channel.clone()),
        ]
    }
}

// ---------------------------------------------------------------------------
// jabber
// ---------------------------------------------------------------------------

/// send a jabber (XMPP) message
pub struct JabberSendAction {
    pub jabber: String,
    pub jid: String,
    pub message: String,
}

impl AmiAction for JabberSendAction {
    fn action_name(&self) -> &str {
        "JabberSend"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Jabber".into(), self.jabber.clone()),
            ("JID".into(), self.jid.clone()),
            ("Message".into(), self.message.clone()),
        ]
    }
}
