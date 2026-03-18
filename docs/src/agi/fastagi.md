# FastAGI Server

## Server Configuration

`AgiServer` uses a builder pattern to configure the TCP listener:

```rust,no_run
use asterisk_agi::{AgiServer, AgiHandler, AgiRequest, AgiChannel, AgiError};

struct MyHandler;
impl AgiHandler for MyHandler {
    async fn handle(&self, _req: AgiRequest, _ch: AgiChannel) -> Result<(), AgiError> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = AgiServer::builder()
        .bind("0.0.0.0:4573")
        .handler(MyHandler)
        .max_connections(100)
        .build()
        .await?;

    server.run().await?;
    Ok(())
}
```

| Method | Default | Description |
|--------|---------|-------------|
| `bind(addr)` | `"0.0.0.0:4573"` | TCP address to listen on |
| `handler(h)` | required | Your `AgiHandler` implementation |
| `max_connections(n)` | unlimited | Maximum concurrent AGI sessions |

The server spawns a new tokio task for each incoming connection.

## AgiHandler Trait

Implement `AgiHandler` to define your call logic:

```rust,no_run
use asterisk_agi::{AgiHandler, AgiRequest, AgiChannel, AgiError};

struct IvrHandler;

impl AgiHandler for IvrHandler {
    async fn handle(
        &self,
        request: AgiRequest,
        mut channel: AgiChannel,
    ) -> Result<(), AgiError> {
        channel.answer().await?;

        // play a prompt and collect up to 4 digits
        let digits = channel.get_data("enter-pin", 5000, 4).await?;
        channel.verbose(&format!("caller entered: {}", digits), 1).await?;

        channel.hangup().await?;
        Ok(())
    }
}
```

The trait requires `Send + Sync + 'static` so the handler can be shared across
connections. The future returned by `handle` must be `Send`.

## AgiRequest

When Asterisk connects, it sends a block of environment variables describing
the call. These are parsed into `AgiRequest` with typed accessors:

| Method | Description |
|--------|-------------|
| `network_script()` | The AGI script path from the dialplan |
| `request_url()` | Full request URL |
| `channel_name()` | Asterisk channel name |
| `channel_type()` | Channel technology (SIP, PJSIP, etc.) |
| `language()` | Channel language |
| `caller_id_num()` | Caller ID number |
| `caller_id_name()` | Caller ID name |
| `dnid()` | Dialed number |
| `context()` | Dialplan context |
| `extension()` | Dialplan extension |
| `priority()` | Dialplan priority |
| `account_code()` | Account code |
| `unique_id()` | Channel unique ID |
| `enhanced()` | Whether enhanced AGI (EAGI) is active |
| `get(key)` | Raw header lookup |

## AgiChannel Commands

`AgiChannel` provides typed methods for each AGI command:

| Method | AGI Command | Description |
|--------|-------------|-------------|
| `answer()` | `ANSWER` | Answer the channel |
| `hangup()` | `HANGUP` | Hang up the channel |
| `stream_file(file, escape)` | `STREAM FILE` | Play an audio file |
| `get_data(file, timeout, max)` | `GET DATA` | Play a file and collect DTMF |
| `say_digits(digits, escape)` | `SAY DIGITS` | Speak digits |
| `say_number(number, escape)` | `SAY NUMBER` | Speak a number |
| `set_variable(name, value)` | `SET VARIABLE` | Set a channel variable |
| `get_variable(name)` | `GET VARIABLE` | Get a channel variable |
| `exec(app, args)` | `EXEC` | Execute a dialplan application |
| `wait_for_digit(timeout)` | `WAIT FOR DIGIT` | Wait for a single DTMF press |
| `channel_status()` | `CHANNEL STATUS` | Query channel state |
| `verbose(msg, level)` | `VERBOSE` | Write to Asterisk CLI |
| `send_command(raw)` | any | Send a raw AGI command string |

All command methods are async and return `Result<AgiResponse>`, where
`AgiResponse` contains the result code and any data returned by Asterisk.
## Example: Full IVR

```rust,no_run
use asterisk_agi::{AgiServer, AgiHandler, AgiRequest, AgiChannel, AgiError};
struct VoicemailHandler {
    greeting: String,
}

impl AgiHandler for VoicemailHandler {
    async fn handle(
        &self,
        request: AgiRequest,
        mut channel: AgiChannel,
    ) -> Result<(), AgiError> {
        channel.answer().await?;
        channel.stream_file(&self.greeting, "#").await?;
        let mailbox = channel.get_data("enter-mailbox", 5000, 4).await?;
        channel.exec("VoiceMail", &format!("{}@default", mailbox)).await?;
        channel.hangup().await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = AgiServer::builder()
        .bind("0.0.0.0:4573")
        .handler(VoicemailHandler {
            greeting: "vm-greeting".to_owned(),
        })
        .build()
        .await?;

    server.run().await?;
    Ok(())
}
```
