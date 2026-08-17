# ARI API reference

The public Rust API is documented in
[`asterisk_rs_ari`](https://docs.rs/asterisk-rs-ari/0.8/asterisk_rs_ari/). Rustdoc is the canonical
inventory of resources, handles, events, transports, media types, and server APIs.

## Find the right API

- [`AriClient`](https://docs.rs/asterisk-rs-ari/0.8/asterisk_rs_ari/struct.AriClient.html) combines
  REST or unified WebSocket requests with the event stream.
- [`resources`](https://docs.rs/asterisk-rs-ari/0.8/asterisk_rs_ari/resources/index.html) groups
  operations by Asterisk resource. Handles bind an ID to a client for follow-up operations.
- [`pending`](https://docs.rs/asterisk-rs-ari/0.8/asterisk_rs_ari/pending/index.html) contains
  subscribe-before-create flows for channels, bridges, and playbacks.
- [`AriEvent`](https://docs.rs/asterisk-rs-ari/0.8/asterisk_rs_ari/event/enum.AriEvent.html) contains
  modeled Stasis events plus forward-compatible unknown-event retention.
- [`media`](https://docs.rs/asterisk-rs-ari/0.8/asterisk_rs_ari/media/index.html) owns
  `chan_websocket` control and audio exchange.
- [`server`](https://docs.rs/asterisk-rs-ari/0.8/asterisk_rs_ari/server/index.html) owns Asterisk 22
  outbound WebSocket sessions.

Use [resources](resources.md) for the handle pattern and [Stasis applications](stasis.md) for event
delivery. Exact operations and signatures belong only in rustdoc so source-module splits cannot
silently stale this guide.
