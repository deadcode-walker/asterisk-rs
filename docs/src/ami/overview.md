# AMI (Asterisk Manager Interface)

The Asterisk Manager Interface (AMI) is a TCP-based protocol for monitoring and
controlling an Asterisk PBX. It listens on port 5038 by default and provides
access to system status, call control, configuration, and real-time events.

## Wire Format

AMI uses a plain-text wire format. Each message is a set of `Key: Value` header
pairs separated by `\r\n`, terminated by an empty line (`\r\n\r\n`).

```text
Action: Ping\r\n
ActionID: 1\r\n
\r\n
```

## Message Types

AMI defines three message types:

- **Actions** are sent by the client to request operations (e.g., `Ping`,
  `Originate`, `Hangup`, `Command`).
- **Responses** are sent by Asterisk in reply to actions, indicating success or
  failure along with any result data.
- **Events** are sent asynchronously by Asterisk when something happens in the
  system (e.g., a new channel is created, a call is hung up, a peer status
  changes).

## Quick Start

```rust,no_run
use asterisk_ami::AmiClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AmiClient::builder()
        .host("127.0.0.1")
        .credentials("admin", "secret")
        .build()
        .await?;

    let resp = client.ping().await?;
    println!("{:?}", resp);
    Ok(())
}
```

The client handles authentication automatically during `build()`. Once
connected, you can send actions and subscribe to events.

## Supported Actions

The crate provides typed action structs for common operations:

| Action | Description |
|--------|-------------|
| `PingAction` | Check connectivity |
| `OriginateAction` | Originate a new call |
| `HangupAction` | Hang up a channel |
| `RedirectAction` | Redirect a channel to a new extension |
| `CommandAction` | Execute a CLI command |
| `GetVarAction` | Get a channel variable |
| `SetVarAction` | Set a channel variable |

For details on building connections, see [Connection & Authentication](./connection.md).
For event handling, see [Events](./events.md).
