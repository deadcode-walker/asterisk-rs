//! typed AMI action types
//!
//! each action serializes to a [`RawAmiMessage`] for transmission

use crate::codec::RawAmiMessage;
use std::sync::atomic::{AtomicU64, Ordering};

/// global action ID counter
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
        (action_id, RawAmiMessage { headers })
    }
}

/// login with plaintext credentials
pub struct LoginAction {
    pub username: String,
    pub secret: String,
}

impl AmiAction for LoginAction {
    fn action_name(&self) -> &str {
        "Login"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Username".into(), self.username.clone()),
            ("Secret".into(), self.secret.clone()),
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
    pub key: String,
}

impl AmiAction for ChallengeLoginAction {
    fn action_name(&self) -> &str {
        "Login"
    }

    fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("AuthType".into(), "md5".into()),
            ("Username".into(), self.username.clone()),
            ("Key".into(), self.key.clone()),
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
