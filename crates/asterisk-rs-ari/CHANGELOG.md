# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- harden credential handling and add AMI header count limit ([#28](https://github.com/deadcode-walker/asterisk-rs/pull/28))


### Fixed
- code quality sweep — error context, panic safety, unused deps ([#26](https://github.com/deadcode-walker/asterisk-rs/pull/26))


### Fixed
- harden ARI config encoding, event-list logic, and resource URL paths
- improve jitter entropy, track dropped calls, preserve critical media events
- percent-encode resource IDs in ARI URL path segments


### Fixed
- harden credential handling, codec safety, and API consistency


### Other
- apply rustfmt formatting fixes


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
