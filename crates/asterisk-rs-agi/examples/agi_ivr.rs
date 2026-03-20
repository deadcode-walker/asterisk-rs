//! FastAGI IVR (interactive voice response) menu example.
//!
//! Demonstrates a realistic phone menu backed by AstDB, with DTMF collection,
//! variable management, and branching logic.
//!
//! # Dialplan entry
//!
//! ```text
//! exten => s,1,AGI(agi://127.0.0.1:4573)
//! ```
//!
//! # Run
//!
//! ```text
//! cargo run --example agi_ivr -p asterisk-rs-agi
//! ```

use asterisk_rs_agi::{AgiChannel, AgiHandler, AgiRequest, AgiServer};

// maximum times to offer the main menu before hanging up
const MAX_RETRIES: u32 = 3;

// AstDB family used for account lookups
const DB_FAMILY: &str = "accounts";

struct IvrHandler;

impl AgiHandler for IvrHandler {
    async fn handle(
        &self,
        request: AgiRequest,
        mut channel: AgiChannel,
    ) -> asterisk_rs_agi::error::Result<()> {
        let caller = request.caller_id().unwrap_or("unknown");
        tracing::info!(caller, "IVR session started");

        channel.answer().await?;

        // stash the caller ID so dialplan extensions can read it back
        channel.set_variable("IVR_CALLER", caller).await?;

        // verify the variable round-trips (useful during development)
        let var_resp = channel.get_variable("IVR_CALLER").await?;
        tracing::info!(result = var_resp.result, "IVR_CALLER variable confirmed");

        // seed AstDB with a demo account balance if not already present
        let balance_resp = channel.database_get(DB_FAMILY, "balance").await?;
        if balance_resp.result == 0 {
            // result == 0 means key not found; write an initial value
            channel.database_put(DB_FAMILY, "balance", "1500").await?;
        }

        let mut retries = 0u32;
        loop {
            if retries >= MAX_RETRIES {
                tracing::info!("max retries reached, hanging up");
                channel.stream_file("goodbye", "#").await?;
                break;
            }

            // play the main menu prompt and collect one digit (5 s timeout)
            channel.stream_file("main-menu", "#").await?;
            let dtmf = channel.get_data("press-1-or-2", 5000, 1).await?;

            // result == -1 means timeout with no digit; result == 0 can mean
            // empty input depending on Asterisk version — treat both as retry
            if dtmf.result <= 0 {
                tracing::info!("no digit received, retrying");
                channel.stream_file("please-try-again", "#").await?;
                retries += 1;
                continue;
            }

            match dtmf.result {
                1 => {
                    // option 1 — account balance
                    if let Err(e) = handle_account_info(&mut channel).await {
                        tracing::error!(error = %e, "account info branch failed");
                    }
                    break;
                }
                2 => {
                    // option 2 — transfer to agent queue
                    if let Err(e) = handle_transfer(&mut channel).await {
                        tracing::error!(error = %e, "transfer branch failed");
                    }
                    break;
                }
                digit => {
                    tracing::info!(digit, "unrecognised menu choice");
                    channel.stream_file("option-invalid", "#").await?;
                    retries += 1;
                }
            }
        }

        channel.hangup(None).await?;
        Ok(())
    }
}

// reads the account balance from AstDB and reads it back to the caller
async fn handle_account_info(channel: &mut AgiChannel) -> asterisk_rs_agi::error::Result<()> {
    channel.stream_file("your-account-balance-is", "#").await?;

    let resp = channel.database_get(DB_FAMILY, "balance").await?;
    if resp.result == 1 {
        // result == 1 means key found; data holds the value as a string
        let balance_str = resp.data.as_deref().unwrap_or("0");
        // parse defensively; fall back to 0 on corrupt data
        let balance: i64 = balance_str.parse().unwrap_or(0);
        tracing::info!(balance, "reading balance to caller");
        channel.say_number(balance, "#").await?;
    } else {
        tracing::info!("no balance record found in AstDB");
        channel
            .stream_file("information-not-available", "#")
            .await?;
    }

    channel.stream_file("thank-you", "#").await?;
    Ok(())
}

// sets a destination variable and transfers the caller via Dial
async fn handle_transfer(channel: &mut AgiChannel) -> asterisk_rs_agi::error::Result<()> {
    tracing::info!("transferring caller to agent queue");
    channel.stream_file("please-hold", "#").await?;

    // store transfer destination as a channel variable for dialplan visibility
    channel
        .set_variable("IVR_TRANSFER_DEST", "Local/agents@queues")
        .await?;

    // exec Dial directly from AGI — hands control back to Asterisk after return
    channel.exec("Dial", "Local/agents@queues,30,tT").await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let (server, shutdown_handle) = AgiServer::builder()
        .bind("0.0.0.0:4573")
        .handler(IvrHandler)
        .max_connections(50)
        .build()
        .await?;

    tracing::info!("FastAGI IVR listening on 0.0.0.0:4573");

    // drive the shutdown handle from a ctrl-c signal so the server exits cleanly
    tokio::spawn(async move {
        if let Ok(()) = tokio::signal::ctrl_c().await {
            tracing::info!("shutdown signal received");
            shutdown_handle.shutdown();
        }
    });

    server.run().await?;
    Ok(())
}
