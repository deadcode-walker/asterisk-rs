# asterisk-rs-core

[![Crates.io](https://img.shields.io/crates/v/asterisk-rs-core)](https://crates.io/crates/asterisk-rs-core)
[![docs.rs](https://img.shields.io/docsrs/asterisk-rs-core)](https://docs.rs/asterisk-rs-core)
[![License](https://img.shields.io/crates/l/asterisk-rs-core)](https://github.com/deadcode-walker/asterisk-rs/blob/main/LICENSE-MIT)

Shared types, error framework, and event bus for asterisk-rs.

This is the foundation crate for the [asterisk-rs](https://crates.io/crates/asterisk-rs) workspace.
Protocol crates (`asterisk-rs-ami`, `asterisk-rs-agi`, `asterisk-rs-ari`) build on top of the
types and infrastructure defined here. Most users should depend on the
[`asterisk-rs`](https://crates.io/crates/asterisk-rs) umbrella crate directly.

## Exports

- **Error types** -- `Error`, `ConnectionError`, `AuthError`, `TimeoutError`, `ProtocolError`
- **Event system** -- `EventBus<E>` and `EventSubscription<E>` for typed pub/sub
- **Reconnect policy** -- `ReconnectPolicy` with exponential backoff and jitter
- **Credentials** -- `Credentials` with redacted `Debug` impl

## Modules

`error`, `event`, `config`, `auth`

## Install

```sh
cargo add asterisk-rs-core
```

## Minimum Supported Rust Version

1.83

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/deadcode-walker/asterisk-rs/blob/main/LICENSE-APACHE) or
[MIT License](https://github.com/deadcode-walker/asterisk-rs/blob/main/LICENSE-MIT) at your option.
