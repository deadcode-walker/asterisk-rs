---
description: "Asterisk protocol domain knowledge. Read when working on AMI, AGI, or ARI code."
globs:
  - "crates/asterisk-ami/**"
  - "crates/asterisk-agi/**"
  - "crates/asterisk-ari/**"
---

# Asterisk Protocols

## AMI (Asterisk Manager Interface)

- TCP line-based protocol on port 5038. Messages are `Key: Value\r\n` pairs terminated by `\r\n\r\n`.
- Three message types: Action (client→server), Response (server→client), Event (server→client async).
- Authentication: `Action: Login` with username/secret, or MD5 challenge-response (`Action: Challenge`).
- Every Action has an `ActionID` header for correlating responses. The server echoes it back.
- Responses can carry multi-line data following `Response: Follows` terminated by `--END COMMAND--`.
- Events are unsolicited and arrive at any time. Must be handled concurrently with action/response pairs.

## AGI (Asterisk Gateway Interface)

- Asterisk connects to an AGI server via TCP (FastAGI, port 4573) and sends environment variables as `key: value\n` lines, terminated by a blank line.
- Commands are single-line text, responses are `xxx result=data` where xxx is a 3-digit status code.
- 200 = success, 510 = invalid command, 520 = usage error.
- Channel is blocked during AGI execution — one command at a time, synchronous.

## ARI (Asterisk REST Interface)

- HTTP REST API + WebSocket event stream. Default port 8088.
- Auth: HTTP Basic (username:password) on every request and WebSocket upgrade.
- WebSocket delivers JSON events for subscribed applications. App subscribes with `?app=name` on the WS URL.
- Resources: channels, bridges, endpoints, device states, mailboxes, sounds, recordings, playbacks.
- Stasis application model: channels enter stasis via dialplan `Stasis(appname)`, controlled via REST.
