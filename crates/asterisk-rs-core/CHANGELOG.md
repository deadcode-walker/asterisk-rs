# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- harden credential handling, codec safety, and API consistency


## [0.2.1](https://github.com/deadcode-walker/asterisk-rs/compare/asterisk-rs-core-v0.2.0...asterisk-rs-core-v0.2.1) - 2026-03-18

### Added

- *(core)* added filtered event subscriptions
- *(core)* added typed Asterisk domain constants

### Other

- rewrote all documentation for production quality

## [0.2.0](https://github.com/deadcode-walker/asterisk-rs/compare/asterisk-rs-core-v0.1.1...asterisk-rs-core-v0.2.0) - 2026-03-18

### Fixed

- *(core)* replaced deterministic jitter with time-based entropy, added tests

### Other

- added #[non_exhaustive] to all public enums

## [0.1.1](https://github.com/deadcode-walker/asterisk-rs/compare/asterisk-rs-core-v0.1.0...asterisk-rs-core-v0.1.1) - 2026-03-18

### Other

- added crate-specific READMEs for sub-crates
