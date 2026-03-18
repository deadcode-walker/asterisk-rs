# Resources

ARI resources are managed through handle objects that bundle a resource ID
with a client reference.

## Handle Pattern

```rust,ignore
use asterisk_rs_ari::resources::channel::{ChannelHandle, originate, OriginateParams};

// originate a channel
let params = OriginateParams {
    endpoint: "PJSIP/100".into(),
    app: Some("my-app".into()),
    ..Default::default()
};
let channel = originate(&client, &params).await?;

// wrap in a handle for operations
let handle = ChannelHandle::new(channel.id, client.clone());
handle.answer().await?;
handle.play("sound:hello-world").await?;
handle.hangup(None).await?;
```

## Available Handles

| Handle | Resource | Key Operations |
|--------|----------|----------------|
| `ChannelHandle` | Channel | answer, hangup, play, record, hold, mute, dtmf, dial, snoop |
| `BridgeHandle` | Bridge | add/remove channel, play, record, moh, video source |
| `PlaybackHandle` | Playback | control, stop |
| `RecordingHandle` | Recording | stop, pause, mute |

## Module Functions

Each resource module also provides free functions for list/get/create:

```rust,ignore
use asterisk_rs_ari::resources::channel;
use asterisk_rs_ari::resources::bridge;

let channels = channel::list(&client).await?;
let bridges = bridge::list(&client).await?;
```

## Asterisk System

The `asterisk` resource module provides system management:

```rust,ignore
use asterisk_rs_ari::resources::asterisk;

let info = asterisk::info(&client, None).await?;
let pong = asterisk::ping(&client).await?;
asterisk::reload_module(&client, "res_pjsip.so").await?;
```

See [Reference](./reference.md) for all operations per resource.
