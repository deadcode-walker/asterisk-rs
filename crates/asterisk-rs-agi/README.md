# asterisk-rs-agi

[![crates.io](https://img.shields.io/crates/v/asterisk-rs-agi)](https://crates.io/crates/asterisk-rs-agi)
[![docs.rs](https://img.shields.io/docsrs/asterisk-rs-agi)](https://docs.rs/asterisk-rs-agi)

Async Rust FastAGI server for the Asterisk Gateway Interface. Answer calls,
collect DTMF, play prompts, query databases, and control call flow.

## Quick Start

```rust,ignore
use asterisk_rs_agi::{AgiServer, AgiHandler, AgiRequest, AgiChannel};

struct MyIvr;

impl AgiHandler for MyIvr {
    async fn handle(&self, req: AgiRequest, mut ch: AgiChannel)
        -> asterisk_rs_agi::error::Result<()>
    {
        ch.answer().await?;
        ch.stream_file("welcome", "").await?;
        let input = ch.get_data("enter-account", 5000, 6).await?;
        ch.verbose(&format!("caller entered: {:?}", input), 1).await?;
        ch.hangup(None).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (server, _shutdown) = AgiServer::builder()
        .bind("0.0.0.0:4573")
        .handler(MyIvr)
        .max_connections(100)
        .build()
        .await?;

    server.run().await?;
    Ok(())
}
```

## Available Commands

### Call control

| Method | Description |
|---|---|
| `answer()` | Answer the channel |
| `hangup(channel)` | Hang up a channel (pass `None` for current) |
| `channel_status(channel)` | Query channel state |
| `set_autohangup(secs)` | Schedule automatic hangup after N seconds |

### Audio

| Method | Description |
|---|---|
| `stream_file(file, escape_digits)` | Play a file; returns digit pressed or empty |
| `control_stream_file(file, escape, fwd_ms, rew_key, pause_key)` | Play with skip/pause controls |
| `get_option(file, escape_digits, timeout_ms)` | Play and wait for a single DTMF digit |
| `record_file(file, format, escape, timeout_ms, beep, silence, max_duration)` | Record audio |
| `set_music(on, class)` | Enable or disable music-on-hold |

### DTMF and speech

| Method | Description |
|---|---|
| `get_data(prompt, timeout_ms, max_digits)` | Collect a DTMF string |
| `wait_for_digit(timeout_ms)` | Wait for a single keypress |
| `say_digits(digits, escape_digits)` | Speak digits individually |
| `say_number(number, escape_digits)` | Speak a number as a cardinal |
| `say_alpha(text, escape_digits)` | Spell out characters |
| `say_phonetic(text, escape_digits)` | Spell using the NATO phonetic alphabet |
| `say_date(epoch_secs, escape_digits)` | Speak a date |
| `say_time(epoch_secs, escape_digits)` | Speak a time |
| `say_datetime(epoch_secs, escape_digits, format, tz)` | Speak a formatted date/time |

### Variables

| Method | Description |
|---|---|
| `set_variable(name, value)` | Set a channel variable |
| `get_variable(name)` | Get a channel variable |
| `get_full_variable(expression)` | Evaluate a dialplan expression (e.g. `${CHANNEL}`) |

### Database

| Method | Description |
|---|---|
| `database_get(family, key)` | Fetch a value from AstDB |
| `database_put(family, key, value)` | Store a value in AstDB |
| `database_del(family, key)` | Delete a key from AstDB |
| `database_deltree(family, keytree)` | Delete a key subtree from AstDB |

### Dialplan

| Method | Description |
|---|---|
| `exec(application, options)` | Run a dialplan application |
| `gosub(context, extension, priority)` | Jump to a dialplan subroutine |
| `set_context(context)` | Change the active dialplan context |
| `set_extension(extension)` | Change the active extension |
| `set_priority(priority)` | Change the active priority |

### Other

| Method | Description |
|---|---|
| `verbose(message, level)` | Write a message to the Asterisk verbose log |
| `set_callerid(callerid)` | Override the caller ID string |
| `noop()` | No-op; useful for keepalive or testing |

## Capabilities

- 60+ async commands covering the full AGI specification
- Handler trait using native async fn (RPITIT, no macro needed)
- Request environment parsing (caller ID, channel, context, extension, and more)
- Automatic channel hangup detection
- Every AGI command with typed async methods
- Configurable concurrency limits via semaphore
- Graceful shutdown via `ShutdownHandle`
- Argument quoting and escaping for special characters

## Documentation

- [API Reference](https://docs.rs/asterisk-rs-agi)
- [User Guide](https://deadcode-walker.github.io/asterisk-rs/)

Part of [asterisk-rs](https://github.com/deadcode-walker/asterisk-rs). MSRV 1.83. MIT/Apache-2.0.
