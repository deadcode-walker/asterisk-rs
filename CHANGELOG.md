# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- `asterisk-rs-ari`: transport abstraction with two modes -- `TransportMode::Http` (default, separate HTTP + WS) and `TransportMode::WebSocket` (unified WS for REST and events, requires Asterisk 20.14.0+)
- `asterisk-rs-ari`: `ExternalMediaParams` typed struct with all ARI fields (encapsulation, transport, connection_type, direction, channel_id, variables) and builder methods
- `asterisk-rs-ari`: expanded `OriginateParams` with channel_id, other_channel_id, originator, formats, variables, label fields
- `asterisk-rs-ari`: resource factory pattern -- `PendingChannel`, `PendingBridge`, `PendingPlayback` for race-free origination with pre-event subscription
- `asterisk-rs-ari`: outbound WebSocket server (`AriServer`) -- accepts incoming WS connections from Asterisk 22+ configured with outbound websockets
- `asterisk-rs-ari`: WebSocket media channel driver (`MediaChannel`) for exchanging raw audio with chan_websocket (Asterisk 20.16.0+)
- `asterisk-rs-ami`: `CallTracker` -- background task correlating AMI events by UniqueID into `CompletedCall` records with channel, linked_id, duration, cause, collected events
- `asterisk-rs`: `Pbx` high-level call abstraction wrapping AmiClient + CallTracker -- `dial()`, `Call::wait_for_answer()`, `Call::hangup()`

### Changed

- `asterisk-rs-ari`: `ChannelHandle::external_media()` now accepts `&ExternalMediaParams` instead of positional arguments (breaking)
- `asterisk-rs-ari`: `AriClient` REST methods dispatch through transport abstraction instead of direct reqwest calls

### Added

- `asterisk-rs-core`: shared error types, event bus with filtered subscriptions, reconnection policy, credentials with secret redaction
- `asterisk-rs-core`: typed domain constants -- hangup causes, channel states, device states, dial statuses, CDR dispositions, peer statuses, queue strategies, extension states, AGI status codes
- `asterisk-rs-ami`: AMI client with typed events and actions covering the full Asterisk 23 protocol surface, MD5 challenge-response auth, automatic reconnection with re-authentication
- `asterisk-rs-ami`: event-collecting actions (`send_collecting`) for multi-event responses (Status, QueueStatus, CoreShowChannels, etc.)
- `asterisk-rs-ami`: filtered event subscriptions, command output capture for `Response: Follows`, connect timeout
- `asterisk-rs-ami`: channel variable extraction -- `ChanVariable(name)` headers parsed into dedicated `HashMap` on `RawAmiMessage` and `AmiResponse`, accessible via `get_variable()`
- `asterisk-rs-ami`: keep-alive ping loop -- configurable periodic `PingAction` via `AmiClientBuilder::ping_interval()` to detect dead TCP connections early
- `asterisk-rs-agi`: FastAGI TCP server with handler trait (RPITIT), all AGI commands with typed async methods, configurable concurrency, graceful shutdown
- `asterisk-rs-ari`: ARI REST client with WebSocket event listener, typed events with metadata (application, timestamp, asterisk_id), resource handles, system management endpoints
- `asterisk-rs-ari`: filtered subscriptions, URL-safe query encoding, HTTP timeouts, WebSocket lifecycle management
- `asterisk-rs`: umbrella crate with feature-gated re-exports (ami, agi, ari)
- `#[non_exhaustive]` on all public enums for forward compatibility
- GitHub Actions CI, security audit, documentation deployment, release automation
- Auto-generated reference documentation from source
- Examples for all three protocols
