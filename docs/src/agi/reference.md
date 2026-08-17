# AGI API reference

The public Rust API is documented in
[`asterisk_rs_agi`](https://docs.rs/asterisk-rs-agi/0.8/asterisk_rs_agi/). Rustdoc is the canonical
inventory of channel commands, request accessors, server configuration, and errors.

## Find the right API

- [`AgiServer`](https://docs.rs/asterisk-rs-agi/0.8/asterisk_rs_agi/struct.AgiServer.html) accepts
  bounded FastAGI sessions and coordinates shutdown.
- [`AgiHandler`](https://docs.rs/asterisk-rs-agi/0.8/asterisk_rs_agi/trait.AgiHandler.html) is the
  application callback for one parsed request and channel session.
- [`AgiRequest`](https://docs.rs/asterisk-rs-agi/0.8/asterisk_rs_agi/struct.AgiRequest.html) provides
  typed access to the FastAGI prelude.
- [`AgiChannel`](https://docs.rs/asterisk-rs-agi/0.8/asterisk_rs_agi/struct.AgiChannel.html) owns the
  command/response exchange. Its rustdoc lists every supported typed command and exact signature.

See [FastAGI server](fastagi.md) for admission, bind, timeout, and shutdown behavior. The crate
examples are the source of complete runnable server programs.
