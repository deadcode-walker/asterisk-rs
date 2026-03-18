---
description: "Asterisk protocol domain knowledge. Read when working on AMI, AGI, or ARI code."
globs:
  - "crates/asterisk-rs-ami/**"
  - "crates/asterisk-rs-agi/**"
  - "crates/asterisk-rs-ari/**"
---

# Asterisk Protocols

## AMI (Asterisk Manager Interface)

- TCP line-based protocol on port 5038. Messages are `Key: Value\r\n` pairs terminated by `\r\n\r\n`.
- Three message types: Action (client→server), Response (server→client), Event (server→client async).
- Authentication: `Action: Login` with username/secret, or MD5 challenge-response (`Action: Challenge`).
- Every Action has an `ActionID` header for correlating responses. The server echoes it back.
- `Response: Follows` carries multi-line command output terminated by `--END COMMAND--`. Lines without `:` in this block are command output, not key-value headers.
- Event-generating actions (Status, CoreShowChannels, QueueStatus, etc.) return events with a matching `ActionID`, terminated by a `*Complete` event (e.g., `StatusComplete`).
- Events may carry `ChanVariable(name)=value` headers for channel variables set on the channel.
- Events are unsolicited and arrive at any time. Must be handled concurrently with action/response pairs.

## AGI (Asterisk Gateway Interface)

- Asterisk connects to an AGI server via TCP (FastAGI, port 4573) and sends environment variables as `key: value\n` lines, terminated by a blank line.
- Commands are single-line text, responses are `xxx result=data` where xxx is a 3-digit status code.
- 200 = success, 510 = invalid command, 520 = usage error.
- Channel is blocked during AGI execution — one command at a time, synchronous.

## ARI (Asterisk REST Interface)

- HTTP REST API + WebSocket event stream. Default port 8088.
- Auth: HTTP Basic (username:password) on every REST request and WebSocket upgrade.
- WebSocket delivers JSON events for subscribed applications. App subscribes with `?app=name` on the WS URL.
- Every event carries base fields: `application`, `timestamp`, `asterisk_id`. Events are wrapped in an `AriMessage` struct containing these fields plus the typed event payload.
- WebSocket reconnects automatically, but dynamic subscriptions created via REST (`POST /applications/{app}/subscription`) are lost on reconnect and must be re-established.
- REST endpoints use Basic Auth on every request; credentials are not session-based.
- Resources: channels, bridges, endpoints, device states, mailboxes, sounds, recordings, playbacks.
- Stasis application model: channels enter stasis via dialplan `Stasis(appname)`, controlled via REST.
