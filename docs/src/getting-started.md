# Getting Started

Add asterisk-rs to your project:

```toml
[dependencies]
asterisk-rs = "0.8"
```

Or pick individual protocols:

```toml
[dependencies]
asterisk-rs = { version = "0.8", default-features = false, features = ["ami"] }
```

Or use crates directly:

```toml
[dependencies]
asterisk-rs-ami = "0.8"
```

## Protocols

| Protocol | Port | Transport | Crate |
|----------|------|-----------|-------|
| AMI | 5038 | TCP | `asterisk-rs-ami` |
| AGI | 4573 | TCP (FastAGI) | `asterisk-rs-agi` |
| ARI | 8088 | HTTP + WebSocket | `asterisk-rs-ari` |

## Domain Types

Common Asterisk constants are available as typed enums in `asterisk_rs_core::types`:
hangup causes, channel states, device states, dial statuses, and more.
See [Domain Types](./types.md) for the full list.

## Requirements

- Rust 1.86 or newer
- tokio runtime
- a C/C++ compiler for the default non-FIPS AWS-LC-backed Rustls provider; CMake, Go, and bindgen are
  not required for this configuration
- A running Asterisk instance for integration

This workspace does not expose or test AWS-LC FIPS mode. A downstream FIPS configuration requires
CMake and Go, and may also require bindgen plus libclang on targets without pre-generated FIPS
bindings; treat that as an unsupported integration until it has its own target-specific CI proof.
