# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- improve jitter entropy, track dropped calls, preserve critical media events
- harden AMI reconnect backoff and fix keep-alive pong tracking
- harden ARI config encoding, event-list logic, and resource URL paths
- secure LoginAction secret and harden AMI codec
- *(agi)* harden command injection, response parsing, OOM, and channel state
- percent-encode resource IDs in ARI URL path segments


### Fixed
- harden credential handling, codec safety, and API consistency


### Other
- apply rustfmt formatting fixes


## [0.1.4](https://github.com/deadcode-walker/asterisk-rs/compare/asterisk-rs-v0.1.3...asterisk-rs-v0.1.4) - 2026-03-18

### Other

- rewrote all documentation to match established pattern
- rewrote all documentation for production quality

## [0.1.3](https://github.com/deadcode-walker/asterisk-rs/compare/asterisk-rs-v0.1.2...asterisk-rs-v0.1.3) - 2026-03-18

### Other

- updated the following local packages: asterisk-rs-core, asterisk-rs-agi, asterisk-rs-ami, asterisk-rs-ari

## [0.1.2](https://github.com/deadcode-walker/asterisk-rs/compare/asterisk-rs-v0.1.1...asterisk-rs-v0.1.2) - 2026-03-18

### Other

- updated the following local packages: asterisk-rs-core, asterisk-rs-agi, asterisk-rs-ami, asterisk-rs-ari

## [0.1.1](https://github.com/deadcode-walker/asterisk-rs/compare/asterisk-rs-v0.1.0...asterisk-rs-v0.1.1) - 2026-03-18

### Fixed

- *(ci)* bumped MSRV to 1.83, added exten to typos allowlist

### Other

- updated changelog, readme, and agents.md with complete protocol coverage
