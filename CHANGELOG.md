# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `asterisk-rs-core`: shared error types, event bus, reconnection policy, credentials
- `asterisk-rs-ami`: AMI client with typed actions (Login, Originate, Hangup, Redirect, Command, GetVar, SetVar, and 109 more), typed events (161 variants covering all Asterisk 23 events), AMI wire protocol codec, MD5 challenge-response auth, automatic reconnection
- `asterisk-rs-agi`: FastAGI TCP server with handler trait, all 47 AGI commands with typed methods, response parsing, configurable concurrency
- `asterisk-rs-ari`: ARI REST client with WebSocket event listener, all 43 ARI event types with typed deserialization, resource handles (ChannelHandle, BridgeHandle, PlaybackHandle, RecordingHandle), resource modules for channels, bridges, endpoints, playbacks, recordings, device state, mailboxes, sounds, applications
- `asterisk-rs`: umbrella crate with feature-gated re-exports (ami, agi, ari)
- GitHub Actions CI, security audit, documentation deployment, release automation
- mdBook user guide and rustdoc API documentation
- Examples for all three protocols
- `asterisk-rs-ami`: complete AMI event coverage — 161 typed variants covering all Asterisk 23 events (core call flow, transfers, bridge, queue, agent, confbridge, meetme, mixmonitor, parking, pickup, device/presence/extension state, PJSIP, CDR/CEL, RTCP, security, system, async AGI, DAHDI, AOC, FAX)
- `asterisk-rs-ami`: complete AMI action coverage — 116 typed action structs covering all Asterisk 23 actions (status, core, database, transfer, bridge, queue, mixmonitor, confbridge, parking, config, PJSIP, dialplan, mailbox/MWI, voicemail, meetme, agent, FAX)
- `asterisk-rs-agi`: complete AGI command coverage — all 47 Asterisk 23 commands with typed methods (database, speech recognition, say commands, flow control, recording, stream control)
- `asterisk-rs-ari`: complete ARI event coverage — all 43 Asterisk 23 events with typed deserialization (Dial, Hold/Unhold, transfers, talking, tone detection, device/presence/endpoint state, contact status, text messages, REST responses)
- `asterisk-rs-ari`: expanded resource operations — channel redirect/ring/silence/dial/external media, bridge MOH/video source control
