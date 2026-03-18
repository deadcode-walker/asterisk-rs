# AGI (Asterisk Gateway Interface)

The Asterisk Gateway Interface (AGI) is a synchronous command protocol for
controlling call flow from external programs. When a call hits an AGI
application in the dialplan, Asterisk connects to the application, sends
environment variables describing the call, and then executes commands sent
back by the application.

## Transports

AGI supports three transport mechanisms:

1. **Process AGI** -- Asterisk spawns a local process and communicates via
   stdin/stdout. Simple but limited to the same machine.
2. **FastAGI** -- Asterisk connects to a TCP server (default port 4573). This
   allows the AGI application to run on a separate host and handle multiple
   concurrent calls.
3. **AsyncAGI** -- Commands are sent via AMI rather than a direct connection.
   Useful for integrating AGI logic into an existing AMI-based application.

## This Crate

The `asterisk-agi` crate implements a **FastAGI server**. It provides:

- `AgiServer` -- a TCP server that accepts incoming FastAGI connections
- `AgiHandler` -- a trait you implement to define call handling logic
- `AgiRequest` -- parsed environment variables from Asterisk
- `AgiChannel` -- typed methods for sending AGI commands

## Quick Start

```rust,no_run
use asterisk_agi::{AgiServer, AgiHandler, AgiRequest, AgiChannel};

struct MyHandler;

impl AgiHandler for MyHandler {
    async fn handle(
        &self,
        request: AgiRequest,
        mut channel: AgiChannel,
    ) -> Result<(), asterisk_agi::AgiError> {
        println!("call from: {}", request.caller_id_num());
        channel.answer().await?;
        channel.stream_file("welcome", "#").await?;
        channel.hangup().await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = AgiServer::builder()
        .bind("0.0.0.0:4573")
        .handler(MyHandler)
        .build()
        .await?;

    server.run().await?;
    Ok(())
}
```

In your Asterisk dialplan, point to this server:

```text
exten => 100,1,AGI(agi://192.168.1.10:4573)
```

For details on the server and handler API, see [FastAGI Server](./fastagi.md).
