# AGI (Asterisk Gateway Interface)

AGI allows external programs to control Asterisk dialplan execution.
This crate implements a FastAGI TCP server that accepts connections
from Asterisk and dispatches them to a handler.

## Quick Start

```rust,ignore
use asterisk_rs_agi::{AgiServer, AgiHandler, AgiRequest, AgiChannel};

struct MyHandler;

impl AgiHandler for MyHandler {
    async fn handle(&self, request: AgiRequest, mut channel: AgiChannel)
        -> asterisk_rs_agi::error::Result<()>
    {
        channel.answer().await?;
        channel.stream_file("hello-world", "").await?;
        channel.hangup(None).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (server, _shutdown) = AgiServer::builder()
        .bind("0.0.0.0:4573")
        // expose only on an isolated/private FastAGI network
        .allow_external_bind(true)
        .handler(MyHandler)
        .max_connections(100)
        .build()
        .await?;

    server.run().await?;
    Ok(())
}
```

FastAGI does not authenticate peers. External binds require an explicit opt-in and must be isolated
with a private network, firewall allowlist, or authenticated TLS proxy. Prefer the default loopback
bind when Asterisk runs on the same host.

## Capabilities

- Every AGI command with typed async methods
- Handler trait using native async fn (RPITIT, no macro needed)
- Request environment parsing from Asterisk
- Configurable concurrency limits
- Optional command deadline, disabled by default for long-running operations
- Graceful shutdown via `ShutdownHandle`

See [FastAGI Server](./fastagi.md) for server details and
[API reference](./reference.md) for the canonical rustdoc command inventory.
