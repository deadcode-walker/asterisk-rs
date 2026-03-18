# Stasis Applications

Stasis is the application model behind ARI. When a channel enters a Stasis
application via the dialplan, Asterisk delegates all control to your code.
You receive events over the WebSocket and issue commands via the REST API.

## Connecting

Use `AriConfigBuilder` to configure the connection:

```rust,no_run
use asterisk_ari::config::AriConfigBuilder;
use asterisk_rs_core::config::ReconnectPolicy;
use std::time::Duration;

let config = AriConfigBuilder::new()
    .host("pbx.example.com")
    .port(8088)
    .username("asterisk")
    .password("secret")
    .app_name("my-stasis-app")
    .secure(true) // use HTTPS/WSS
    .reconnect(ReconnectPolicy::exponential(
        Duration::from_secs(1),
        Duration::from_secs(30),
    ))
    .build()
    .unwrap();
```

| Method | Default | Description |
|--------|---------|-------------|
| `host(h)` | `"127.0.0.1"` | Asterisk server hostname |
| `port(p)` | `8088` | HTTP port |
| `username(u)` | required | ARI username |
| `password(p)` | required | ARI password |
| `app_name(name)` | required | Stasis application name |
| `secure(bool)` | `false` | Use HTTPS and WSS |
| `reconnect(policy)` | exponential | WebSocket reconnection strategy |

## Event Loop

After connecting, subscribe to receive events:

```rust,no_run
use asterisk_ari::{AriClient, AriEvent};

async fn run(client: AriClient) {
    let mut sub = client.subscribe();

    while let Ok(event) = sub.recv().await {
        match event {
            AriEvent::StasisStart(data) => {
                println!("channel entered: {:?}", data.channel);
            }
            AriEvent::StasisEnd(data) => {
                println!("channel left: {:?}", data.channel);
            }
            AriEvent::ChannelDtmfReceived(data) => {
                println!("DTMF: {}", data.digit);
            }
            _ => {}
        }
    }
}
```

## Event Types

| Variant | Description |
|---------|-------------|
| `StasisStart` | Channel entered the Stasis application |
| `StasisEnd` | Channel left the Stasis application |
| `ChannelCreated` | A channel was created |
| `ChannelDestroyed` | A channel was destroyed |
| `ChannelStateChange` | Channel state changed (ringing, up, etc.) |
| `ChannelDtmfReceived` | A DTMF digit was received |
| `ChannelHangupRequest` | Hangup was requested |
| `ChannelVarset` | A channel variable was set |
| `BridgeCreated` | A bridge was created |
| `BridgeDestroyed` | A bridge was destroyed |
| `ChannelEnteredBridge` | A channel joined a bridge |
| `ChannelLeftBridge` | A channel left a bridge |
| `PlaybackStarted` | A playback operation began |
| `PlaybackFinished` | A playback operation completed |
| `RecordingStarted` | A recording began |
| `RecordingFinished` | A recording completed |
| `Unknown` | Any event not covered above |

## Example: Stasis Application

A complete application that answers calls, plays a prompt, and hangs up:

```rust,no_run
use asterisk_ari::{AriClient, AriConfig, AriEvent};
use asterisk_ari::config::AriConfigBuilder;
use asterisk_ari::resources::channel::ChannelHandle;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AriConfigBuilder::new()
        .host("127.0.0.1")
        .port(8088)
        .username("asterisk")
        .password("secret")
        .app_name("hello-world")
        .build()?;

    let client = AriClient::connect(config).await?;
    let mut sub = client.subscribe();

    while let Ok(event) = sub.recv().await {
        match event {
            AriEvent::StasisStart(data) => {
                let ch = ChannelHandle::new(&client, &data.channel.id);
                ch.answer().await?;
                ch.play("sound:hello-world", "en", 0, 0).await?;
            }
            AriEvent::PlaybackFinished(_) => {
                // playback done, hang up would go here
            }
            AriEvent::StasisEnd(_) => {
                println!("channel left stasis");
            }
            _ => {}
        }
    }

    Ok(())
}
```

In the Asterisk dialplan:

```text
exten => 100,1,Stasis(hello-world)
```
