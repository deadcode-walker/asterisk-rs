# asterisk-rs-core

[![crates.io](https://img.shields.io/crates/v/asterisk-rs-core)](https://crates.io/crates/asterisk-rs-core)
[![docs.rs](https://img.shields.io/docsrs/asterisk-rs-core)](https://docs.rs/asterisk-rs-core)

Shared foundation for the asterisk-rs ecosystem.

Provides error types, event bus, reconnection policy, credentials, and
typed domain constants (hangup causes, channel states, device states, etc.)
used across the AMI, AGI, and ARI protocol crates.

This crate is a dependency of the protocol crates. You don't need to depend
on it directly unless you're building custom protocol integrations.

Part of [asterisk-rs](https://github.com/deadcode-walker/asterisk-rs). MSRV 1.83. MIT/Apache-2.0.
