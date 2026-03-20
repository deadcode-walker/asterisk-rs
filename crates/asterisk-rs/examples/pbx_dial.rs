//! Example: dial a call via the high-level Pbx abstraction.
//!
//! Connects to Asterisk AMI, originates a call from PJSIP/100 to extension
//! 200, waits for the far end to answer, then hangs up and prints the
//! completed-call record (CDR-equivalent).
//!
//! Prerequisites:
//!   - Asterisk running with AMI enabled on 127.0.0.1:5038
//!   - AMI user "admin" with secret "secret" and read/write all
//!   - PJSIP endpoint 100 registered and dialplan routing extension 200
//!
//! Usage: cargo run -p asterisk-rs --example pbx_dial

use std::time::Duration;

use asterisk_rs::ami::AmiClient;
use asterisk_rs::pbx::{DialOptions, Pbx, PbxError};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(5038)
        .credentials("admin", "secret")
        .timeout(Duration::from_secs(10))
        .event_capacity(2048)
        .build()
        .await?;

    tracing::info!("connected to AMI");

    let mut pbx = Pbx::new(client);

    let options = DialOptions::new()
        .caller_id("Rust PBX <100>")
        .timeout_ms(30_000);

    tracing::info!(from = "PJSIP/100", to = "200", "dialing");

    let call = match pbx.dial("PJSIP/100", "200", Some(options)).await {
        Ok(c) => c,
        Err(PbxError::CallFailed { cause, cause_txt }) => {
            tracing::error!(cause, cause_txt, "call failed before answer");
            pbx.shutdown();
            return Ok(());
        }
        Err(PbxError::Timeout) => {
            tracing::error!("originate timed out");
            pbx.shutdown();
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };

    tracing::info!(
        channel = %call.channel,
        unique_id = %call.unique_id,
        "call originated, waiting for answer"
    );

    match call.wait_for_answer(Duration::from_secs(30)).await {
        Ok(()) => {
            tracing::info!(channel = %call.channel, "call answered");
        }
        Err(PbxError::CallFailed { cause, cause_txt }) => {
            tracing::error!(cause, cause_txt, "remote end rejected the call");
            pbx.shutdown();
            return Ok(());
        }
        Err(PbxError::Timeout) => {
            tracing::error!("no answer within 30 seconds, hanging up");
            call.hangup().await?;
            pbx.shutdown();
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    }

    // call is up — do work here, then hang up
    call.hangup().await?;
    tracing::info!(channel = %call.channel, "hung up");

    // drain one completed-call record; the tracker correlates all AMI events
    // that occurred during the channel's lifetime into a single struct
    if let Some(completed) = pbx.next_completed_call().await {
        tracing::info!(
            channel          = %completed.channel,
            unique_id        = %completed.unique_id,
            linked_id        = %completed.linked_id,
            duration_secs    = completed.duration.as_secs(),
            cause            = completed.cause,
            cause_txt        = %completed.cause_txt,
            event_count      = completed.events.len(),
            "call record"
        );
    }

    pbx.shutdown();
    Ok(())
}
