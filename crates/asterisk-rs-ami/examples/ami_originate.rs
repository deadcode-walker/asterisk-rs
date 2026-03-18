//! Example: originate a call via AMI.
//!
//! Connects to Asterisk AMI, logs in, and originates a call
//! from SIP/100 to extension 200 in the default context.
//!
//! Usage: cargo run --example ami_originate

use asterisk_rs_ami::action::OriginateAction;
use asterisk_rs_ami::AmiClient;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(5038)
        .credentials("admin", "secret")
        .timeout(Duration::from_secs(10))
        .build()
        .await?;

    println!("connected and logged in");

    let originate = OriginateAction::new("SIP/100")
        .context("default")
        .extension("200")
        .priority(1)
        .timeout_ms(30000)
        .caller_id("Rust AMI <100>")
        .async_originate(true);

    let response = client.originate(originate).await?;
    println!("originate response: {:?}", response);

    client.disconnect().await?;
    Ok(())
}
