# Asterisk 22.9.0 protocol contracts

Status: pinned external reference metadata. Protocol maintainers own this reference and refresh it
when the supported Asterisk branch changes or a security/correctness fix requires a newer fixture.

The supported protocol baseline is the signed Git tag `22.9.0`, peeled commit
`da123773c723ed1263ff74569544f7ee84626c1a` in
[`asterisk/asterisk`](https://github.com/asterisk/asterisk/tree/22.9.0). The ARI contract is the eleven
JSON documents under `rest-api/api-docs/`; the media WebSocket contract is
`channels/chan_websocket.c`. Their recorded SHA-256 digests give maintainers an exact identity to
verify when refreshing the pin. The generated `asterisk-22.9.0-inventory.json` records every
upstream route and model; `asterisk-22.9.0-local.json` records every local public resource operation
and model symbol. `scripts/check_protocol_contracts.py` validates both inventories against their
declared counts and exactly compares the deliberately supported local surface to the Rust source.
`just protocol-contracts-upstream` independently downloads the files at the pinned commit, verifies
every digest, regenerates the route and model inventory in memory, and requires an exact match with
the checked-in inventory. The ordinary `just harness` gate remains deterministic and offline.

This crate does not claim the complete ARI surface. The upstream fixture contains 102 REST
operations and 82 models. The local manifest records implemented resource groups and typed events;
unknown ARI events must retain their type and raw payload so applications remain forward-compatible.
Changing a route, model, or media command requires updating implementation, external behavior tests,
the manifest, and this reference in one slice.

The upstream media driver defaults to plaintext control messages. JSON control is selected through
the external-media `transport_data=f(json)` contract. JSON-only commands and correlated events must
therefore never be exposed as though they worked on an unspecified/default connection.
