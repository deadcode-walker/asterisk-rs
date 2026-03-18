# Getting Started

## Installation

Add `asterisk-rs` to your project:

```sh
cargo add asterisk-rs
```

Or pick individual protocol crates:

```sh
cargo add asterisk-rs-ami    # AMI only
cargo add asterisk-rs-agi    # AGI only
cargo add asterisk-rs-ari    # ARI only
```

## Prerequisites

You need a running Asterisk instance. The crate supports Asterisk 16+ (LTS and current releases).

## Protocols

Asterisk exposes three interfaces:

| Protocol | Port | Transport | Use Case |
|----------|------|-----------|----------|
| AMI | 5038 | TCP | Server management, call control, event monitoring |
| AGI | 4573 | TCP (FastAGI) | Dialplan scripting, IVR logic |
| ARI | 8088 | HTTP + WebSocket | Application development, full call control |

Each protocol has its own crate with independent documentation. See the sidebar for protocol-specific guides.

## Quick Example

Connect to AMI and originate a call:

```rust,no_run
use asterisk_ami::AmiClient;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(5038)
        .credentials("admin", "secret")
        .build()
        .await?;

    client.originate()
        .channel("SIP/100")
        .context("default")
        .extension("200")
        .priority(1)
        .timeout(Duration::from_secs(30))
        .send()
        .await?;

    Ok(())
}
```
