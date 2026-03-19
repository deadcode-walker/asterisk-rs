# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- *(ari,ami)* added resource factory pattern and PBX abstraction
- *(ari,ami)* added ExternalMediaParams, expanded OriginateParams, added CallTracker
- *(ari)* added outbound WebSocket server and media channel driver
- *(ari)* added REST-over-WebSocket transport mode


### Changed
- *(tests)* moved inline tests to dedicated /tests/ crate


### Documentation
- updated changelogs for all new features


### Fixed
- resolved clippy warnings, fmt, and adapted existing tests


### Other
- cargo fmt
- consolidated releases, CI fixes, documentation updates


### Testing
- added comprehensive tests for all new features
- restructure test architecture + add massive coverage
- add 120 unit tests across all crates (Wave 1)


### Added
- *(ari)* added outbound WebSocket server and media channel driver
- *(ari,ami)* added resource factory pattern and PBX abstraction
- *(ari)* added REST-over-WebSocket transport mode
- *(ari,ami)* added ExternalMediaParams, expanded OriginateParams, added CallTracker


### Changed
- *(tests)* moved inline tests to dedicated /tests/ crate


### Documentation
- updated changelogs for all new features


### Fixed
- resolved clippy warnings, fmt, and adapted existing tests


### Other
- consolidated releases, CI fixes, documentation updates


### Testing
- added comprehensive tests for all new features
- restructure test architecture + add massive coverage
- add 120 unit tests across all crates (Wave 1)


### Added
- *(ari,ami)* added resource factory pattern and PBX abstraction
- *(ari,ami)* added ExternalMediaParams, expanded OriginateParams, added CallTracker


### Changed
- *(tests)* moved inline tests to dedicated /tests/ crate


### Documentation
- updated changelogs for all new features


### Other
- cargo fmt


### Testing
- added comprehensive tests for all new features
- restructure test architecture + add massive coverage
- add 120 unit tests across all crates (Wave 1)


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
