//! high-level PBX abstraction for call management over AMI
//!
//! wraps [`AmiClient`](asterisk_rs_ami::AmiClient) with call lifecycle
//! tracking and convenience methods for common telephony operations

use std::time::Duration;

use asterisk_rs_ami::action::{HangupAction, OriginateAction};
use asterisk_rs_ami::event::AmiEvent;
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
    pub async fn hangup(
        &self,
    ) -> asterisk_rs_ami::error::Result<asterisk_rs_ami::response::AmiResponse> {
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
    /// uses async originate so the call is queued immediately.
    /// waits for the OriginateResponse event to get the actual
    /// channel name and unique_id.
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
            .priority(1)
            .async_originate(true);

        if let Some(ref cid) = opts.caller_id {
            action = action.caller_id(cid);
        }
        if let Some(ms) = opts.timeout_ms {
            action = action.timeout_ms(ms);
        }

        // subscribe to OriginateResponse before sending the action
        // so we don't miss the event
        let mut orig_sub = self
            .client
            .subscribe_filtered(move |e| matches!(e, AmiEvent::OriginateResponse { .. }));

        self.client.originate(action).await?;

        // wait for the OriginateResponse event with a timeout
        let originate_timeout =
            Duration::from_secs(opts.timeout_ms.map(|ms| ms / 1000 + 5).unwrap_or(35));

        let event = tokio::time::timeout(originate_timeout, async {
            loop {
                match orig_sub.recv().await {
                    Some(AmiEvent::OriginateResponse {
                        channel,
                        unique_id,
                        response,
                        ..
                    }) => {
                        return Ok((channel, unique_id, response));
                    }
                    Some(_) => continue,
                    None => return Err(PbxError::Disconnected),
                }
            }
        })
        .await
        .map_err(|_| PbxError::Timeout)??;

        let (channel, unique_id, response) = event;

        if response.eq_ignore_ascii_case("failure") {
            return Err(PbxError::CallFailed {
                cause: 0,
                cause_txt: "originate failed".to_owned(),
            });
        }

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
