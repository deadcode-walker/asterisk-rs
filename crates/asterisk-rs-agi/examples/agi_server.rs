use asterisk_rs_agi::{AgiChannel, AgiHandler, AgiRequest, AgiServer};

struct MyHandler;

impl AgiHandler for MyHandler {
    async fn handle(
        &self,
        request: AgiRequest,
        mut channel: AgiChannel,
    ) -> asterisk_rs_agi::error::Result<()> {
        tracing::info!(channel = ?request.channel(), "new AGI session");
        channel.answer().await?;
        channel.verbose("Hello from Rust AGI!", 1).await?;
        channel.stream_file("hello-world", "#").await?;
        channel.hangup(None).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let server = AgiServer::builder()
        .bind("0.0.0.0:4573")
        .handler(MyHandler)
        .max_connections(100)
        .build()
        .await?;

    tracing::info!("FastAGI server listening on 0.0.0.0:4573");
    server.run().await?;

    Ok(())
}
