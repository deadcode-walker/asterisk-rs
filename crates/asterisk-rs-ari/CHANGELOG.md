# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- transport abstraction with `TransportMode::Http` (default) and `TransportMode::WebSocket` (unified WS, requires Asterisk 20.14.0+)
- `ExternalMediaParams` typed struct with all ARI fields and builder methods
- expanded `OriginateParams` with channel_id, other_channel_id, originator, formats, variables, label
- resource factory: `PendingChannel`, `PendingBridge`, `PendingPlayback` for race-free origination
- outbound WebSocket server (`AriServer`) for Asterisk 22+ outbound WS connections
- WebSocket media channel driver (`MediaChannel`) for chan_websocket audio exchange (Asterisk 20.16.0+)
- `AriClient::channel()`, `bridge()`, `playback()` factory methods
- `AriClient::config()` accessor

### Changed

- `ChannelHandle::external_media()` now accepts `&ExternalMediaParams` instead of positional arguments (breaking)
- REST methods dispatch through transport abstraction instead of direct reqwest

## [0.3.1](https://github.com/deadcode-walker/asterisk-rs/compare/asterisk-rs-ari-v0.3.0...asterisk-rs-ari-v0.3.1) - 2026-03-18

### Added

- *(core)* added filtered event subscriptions
- *(ari)* added AriMessage wrapper with application, timestamp, asterisk_id

### Other

- rewrote all documentation to match established pattern
- rewrote all documentation for production quality

## [0.3.0](https://github.com/deadcode-walker/asterisk-rs/compare/asterisk-rs-ari-v0.2.1...asterisk-rs-ari-v0.3.0) - 2026-03-18

### Fixed

- *(ari)* added URL encoding, HTTP timeouts, WebSocket cleanup

### Other

- added #[non_exhaustive] to all public enums

## [0.2.1](https://github.com/deadcode-walker/asterisk-rs/compare/asterisk-rs-ari-v0.2.0...asterisk-rs-ari-v0.2.1) - 2026-03-18

### Added

- *(ari)* added remaining endpoint and application operations
- *(ari)* added asterisk system resource module with all 16 endpoints

### Other

- added crate-specific READMEs for sub-crates

## [0.2.0](https://github.com/deadcode-walker/asterisk-rs/compare/asterisk-rs-ari-v0.1.0...asterisk-rs-ari-v0.2.0) - 2026-03-18

### Added

- *(ari)* added complete ARI event and resource coverage for Asterisk 23
