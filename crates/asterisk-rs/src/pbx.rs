//! high-level PBX abstraction for call management over AMI
//!
//! wraps [`AmiClient`](asterisk_rs_ami::AmiClient) with call lifecycle
//! tracking and convenience methods for common telephony operations

use std::time::Duration;

use asterisk_rs_ami::action::{HangupAction, OriginateAction};
use asterisk_rs_ami::event::AmiEvent;
use asterisk_rs_ami::response::AmiResponse;
use asterisk_rs_ami::tracker::{CallTracker, CompletedCall};
use asterisk_rs_ami::AmiClient;
use tokio::sync::mpsc;

/// a live call being tracked by the PBX
///
/// wraps a channel name and unique_id with the AMI client for
/// issuing commands and tracking events
#[derive(Debug, Clone)]
pub struct Call {
    /// channel name (e.g. "PJSIP/100-00000001")
    pub channel: String,
    /// per-channel unique identifier
    pub unique_id: String,
    client: AmiClient,
}

impl Call {
    /// hang up this call
    pub async fn hangup(&self) -> asterisk_rs_ami::error::Result<AmiResponse> {
        self.client.hangup(HangupAction::new(&self.channel)).await
    }

    /// wait for this channel to reach "Up" state (answered)
    ///
    /// listens for Newstate events with channel_state_desc "Up".
    /// returns Err if the channel hangs up before answering
    pub async fn wait_for_answer(&self, timeout: Duration) -> Result<(), PbxError> {
        let uid = self.unique_id.clone();
        let mut sub = self.client.subscribe_filtered(move |e| match e {
            AmiEvent::Newstate { unique_id, .. } | AmiEvent::Hangup { unique_id, .. } => {
                *unique_id == uid
            }
            _ => false,
        });

        let result = tokio::time::timeout(timeout, async {
            loop {
                match sub.recv().await {
                    Some(AmiEvent::Newstate {
                        channel_state_desc, ..
                    }) => {
                        if channel_state_desc == "Up" {
                            return Ok(());
                        }
                    }
                    Some(AmiEvent::Hangup {
                        cause, cause_txt, ..
                    }) => {
                        return Err(PbxError::CallFailed { cause, cause_txt });
                    }
                    None => return Err(PbxError::Disconnected),
                    _ => {}
                }
            }
        })
        .await;

        match result {
            Ok(inner) => inner,
            Err(_) => Err(PbxError::Timeout),
        }
    }
}

/// options for originating a call
#[derive(Debug, Clone, Default)]
pub struct DialOptions {
    /// caller ID to present
    pub caller_id: Option<String>,
    /// maximum time to wait for answer in milliseconds
    pub timeout_ms: Option<u64>,
    /// channel variables to set
    pub variables: Option<std::collections::HashMap<String, String>>,
}

impl DialOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn caller_id(mut self, cid: impl Into<String>) -> Self {
        self.caller_id = Some(cid.into());
        self
    }

    /// set max wait time in milliseconds (matches Asterisk Originate timeout)
    pub fn timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = Some(ms);
        self
    }
}

/// errors from PBX operations
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PbxError {
    #[error("AMI error: {0}")]
    Ami(#[from] asterisk_rs_ami::AmiError),

    #[error("call failed: {cause} ({cause_txt})")]
    CallFailed { cause: u32, cause_txt: String },

    #[error("operation timed out")]
    Timeout,

    #[error("client disconnected")]
    Disconnected,
}

/// high-level PBX abstraction wrapping an AMI client
///
/// provides convenient methods for common telephony operations
/// with built-in call tracking via [`CallTracker`]
#[derive(Debug)]
pub struct Pbx {
    client: AmiClient,
    tracker: CallTracker,
    completed_rx: mpsc::Receiver<CompletedCall>,
}

impl Pbx {
    /// create a new PBX abstraction wrapping an AMI client
    pub fn new(client: AmiClient) -> Self {
        let (tracker, completed_rx) = client.call_tracker();
        Self {
            client,
            tracker,
            completed_rx,
        }
    }

    /// originate a call from one endpoint to another
    ///
    /// creates a channel to `from`, then connects it to `to` via the dialplan.
    /// returns a [`Call`] handle for tracking and controlling the call
    pub async fn dial(
        &self,
        from: impl Into<String>,
        to: impl Into<String>,
        options: Option<DialOptions>,
    ) -> Result<Call, PbxError> {
        let from = from.into();
        let to = to.into();
        let opts = options.unwrap_or_default();

        let mut action = OriginateAction::new(&from)
            .extension(&to)
            .context("default")
            .priority(1);

        if let Some(ref cid) = opts.caller_id {
            action = action.caller_id(cid);
        }
        if let Some(ms) = opts.timeout_ms {
            action = action.timeout_ms(ms);
        }

        let response = self.client.originate(action).await?;

        // extract unique_id from response headers, default to empty if absent
        let unique_id = response
            .get("Uniqueid")
            .map(|s| s.to_owned())
            .unwrap_or_default();

        // extract channel name from response, fall back to the requested endpoint
        let channel = response
            .get("Channel")
            .map(|s| s.to_owned())
            .unwrap_or_else(|| from.clone());

        Ok(Call {
            channel,
            unique_id,
            client: self.client.clone(),
        })
    }

    /// receive the next completed call record
    pub async fn next_completed_call(&mut self) -> Option<CompletedCall> {
        self.completed_rx.recv().await
    }

    /// access the underlying AMI client
    pub fn client(&self) -> &AmiClient {
        &self.client
    }

    /// shut down the call tracker
    pub fn shutdown(self) {
        self.tracker.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dial_options_default() {
        let opts = DialOptions::new();
        assert!(opts.caller_id.is_none());
        assert!(opts.timeout_ms.is_none());
        assert!(opts.variables.is_none());
    }

    #[test]
    fn test_dial_options_builder() {
        let opts = DialOptions::new()
            .caller_id("Test <1234>")
            .timeout_ms(30000);

        assert_eq!(
            opts.caller_id.as_deref(),
            Some("Test <1234>")
        );
        assert_eq!(opts.timeout_ms, Some(30000));
    }

    #[test]
    fn test_pbx_error_display() {
        let err = PbxError::Timeout;
        assert_eq!(err.to_string(), "operation timed out");

        let err = PbxError::Disconnected;
        assert_eq!(err.to_string(), "client disconnected");

        let err = PbxError::CallFailed {
            cause: 16,
            cause_txt: "Normal Clearing".to_owned(),
        };
        assert_eq!(
            err.to_string(),
            "call failed: 16 (Normal Clearing)"
        );
    }
}
