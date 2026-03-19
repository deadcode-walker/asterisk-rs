//! Production Rust client for Asterisk AMI, AGI, and ARI.
//!
//! This crate re-exports the individual protocol crates under feature flags.
//! By default, all protocols are enabled. Disable defaults and pick what you need:
//!
//! ```toml
//! [dependencies]
//! asterisk-rs = { version = "0.1", default-features = false, features = ["ami"] }
//! ```
//!
//! Or use individual crates directly:
//!
//! ```toml
//! [dependencies]
//! asterisk-ami = "0.1"
//! ```

pub use asterisk_rs_core as core;

#[cfg(feature = "ami")]
pub use asterisk_rs_ami as ami;

#[cfg(feature = "agi")]
pub use asterisk_rs_agi as agi;

#[cfg(feature = "ari")]
pub use asterisk_rs_ari as ari;

#[cfg(feature = "ami")]
pub mod pbx;
