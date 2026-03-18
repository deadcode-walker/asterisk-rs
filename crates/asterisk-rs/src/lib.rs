//! Unified Rust client for Asterisk AMI, AGI, and ARI.
//!
//! This crate re-exports the individual protocol crates under feature flags.
//! For now, only the core types are available. Protocol crates will be added
//! as they are implemented.

pub use asterisk_rs_core as core;
