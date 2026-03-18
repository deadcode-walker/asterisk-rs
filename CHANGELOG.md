# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `asterisk-rs-core`: shared error types, event bus, reconnection policy, credentials
- `asterisk-ami`: AMI client with typed actions (Login, Originate, Hangup, Redirect, Command, GetVar, SetVar), typed events (NewChannel, Hangup, Newstate, Dial, DTMF, Bridge, PeerStatus, FullyBooted), AMI wire protocol codec, MD5 challenge-response auth, automatic reconnection
- `asterisk-agi`: FastAGI TCP server with handler trait, typed AGI commands (answer, hangup, stream_file, get_data, say_digits, say_number, set/get_variable, exec, wait_for_digit, verbose), response parsing, configurable concurrency
- `asterisk-ari`: ARI REST client with WebSocket event listener, typed events (StasisStart, StasisEnd, ChannelCreated, ChannelDtmfReceived, BridgeCreated, PlaybackStarted, etc.), resource handles (ChannelHandle, BridgeHandle, PlaybackHandle, RecordingHandle), resource modules for channels, bridges, endpoints, playbacks, recordings, device state, mailboxes, sounds, applications
- `asterisk-rs`: umbrella crate with feature-gated re-exports (ami, agi, ari)
- GitHub Actions CI, security audit, documentation deployment, release automation
- mdBook user guide and rustdoc API documentation
- Examples for all three protocols
