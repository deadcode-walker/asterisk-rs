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
- harden AMI reconnect backoff and fix keep-alive pong tracking
- harden ARI config encoding, event-list logic, and resource URL paths
- improve jitter entropy, track dropped calls, preserve critical media events
- secure LoginAction secret and harden AMI codec


### Fixed
- harden credential handling, codec safety, and API consistency


## [0.4.0](https://github.com/deadcode-walker/asterisk-rs/compare/asterisk-rs-ami-v0.3.0...asterisk-rs-ami-v0.4.0) - 2026-03-18

### Added

- *(ami)* added keep-alive ping loop with configurable interval
- *(ami)* added channel variable extraction from ChanVariable headers
- *(ami)* added event-generating action collection via send_collecting
- *(core)* added filtered event subscriptions

### Fixed

- *(ami)* added command output parsing, Serialize, connect timeout

### Other

- updated AGENTS.md, CHANGELOG, and AMI README for channel variables and ping loop
- rewrote all documentation to match established pattern
- rewrote all documentation for production quality

## [0.3.0](https://github.com/deadcode-walker/asterisk-rs/compare/asterisk-rs-ami-v0.2.1...asterisk-rs-ami-v0.3.0) - 2026-03-18

### Fixed

- *(ami)* re-authenticate on reconnect, added PartialEq derives

### Other

- added #[non_exhaustive] to all public enums

## [0.2.1](https://github.com/deadcode-walker/asterisk-rs/compare/asterisk-rs-ami-v0.2.0...asterisk-rs-ami-v0.2.1) - 2026-03-18

### Added

- *(ami)* added remaining 37 AMI actions for complete Asterisk 23 coverage

### Other

- added crate-specific READMEs for sub-crates

## [0.2.0](https://github.com/deadcode-walker/asterisk-rs/compare/asterisk-rs-ami-v0.1.0...asterisk-rs-ami-v0.2.0) - 2026-03-18

### Added

- *(ami)* added complete AMI action coverage for Asterisk 23
- *(ami)* added complete AMI event coverage for Asterisk 23
